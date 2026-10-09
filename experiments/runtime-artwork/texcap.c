/* Original coding-agent contribution, 2026-10-09. Runtime GL pixels only. */
#define _GNU_SOURCE
#define GL_GLEXT_PROTOTYPES
#include <GL/gl.h>
#include <GL/glext.h>
#include <GL/glx.h>
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>
#define MAX_IMAGE (64ull*1024*1024)
#define MAX_TOTAL (256ull*1024*1024)
#define MAX_EVENTS 4096
static void (*real_upload)(GLenum,GLint,GLint,GLsizei,GLsizei,GLint,GLenum,GLenum,const void*);
static GLenum (*real_error)(void);
static pthread_once_t once=PTHREAD_ONCE_INIT;
static pthread_mutex_t lock=PTHREAD_MUTEX_INITIALIZER;
static _Thread_local int inside;
static GLXContext owner;
static pthread_t owner_thread;
static GLenum pending[16];
static unsigned n_pending, seq;
static uint64_t total;
static int enabled, pbo_ok;
static char session[4096], vendor[256], renderer[256], version[256];
static void resolve(void) {
    *(void**)(&real_upload)=dlsym(RTLD_NEXT,"glTexImage2D");
    *(void**)(&real_error)=dlsym(RTLD_NEXT,"glGetError");
    if (!real_upload || !real_error) { fputs("texcap: missing real GL symbol\n",stderr); _exit(125); }
    const char *dir=getenv("TEXCAP_DIR");
    if (!dir || !*dir) return;
    int n=snprintf(session,sizeof session,"%s/session-%ld-XXXXXX",dir,(long)getpid());
    if (n<0 || (size_t)n>=sizeof session || !mkdtemp(session)) {
        perror("texcap: session creation failed; capture disabled"); return;
    }
    enabled=1;
    fprintf(stderr,"texcap: output=%s\n",session);
}
static int is_owner(void) {
    return owner && pthread_equal(owner_thread,pthread_self()) && glXGetCurrentContext()==owner;
}
static GLenum drain(int preserve) {
    GLenum first=0,e;
    while ((e=real_error())!=GL_NO_ERROR) {
        if (!first) first=e;
        if (preserve) {
            unsigned i=0; while(i<n_pending && pending[i]!=e) i++;
            if (i==n_pending) {
                if(n_pending==16) { fputs("texcap: GL error queue overflow\n",stderr); _exit(125); }
                pending[n_pending++]=e;
            }
        }
    }
    return first;
}
GLenum glGetError(void) {
    pthread_once(&once,resolve);
    if (!inside && is_owner() && n_pending) {
        GLenum e=pending[0]; memmove(pending,pending+1,--n_pending*sizeof *pending); return e;
    }
    return real_error();
}
static int extension(const char *all,const char *name) {
    size_t n=strlen(name); const char *p=all;
    while(p && (p=strstr(p,name))) {
        if((p==all || p[-1]==' ') && (p[n]==0 || p[n]==' ')) return 1;
        p+=n;
    } return 0;
}
static void context_info(void) {
    const GLubyte *s=glGetString(GL_VERSION); int major=0,minor=0;
    snprintf(version,sizeof version,"%s",s?(const char*)s:"");
    s=glGetString(GL_VENDOR); snprintf(vendor,sizeof vendor,"%s",s?(const char*)s:"");
    s=glGetString(GL_RENDERER); snprintf(renderer,sizeof renderer,"%s",s?(const char*)s:"");
    sscanf(version,"%d.%d",&major,&minor);
    pbo_ok=major>2 || (major==2 && minor>=1);
    if(!pbo_ok) {
        const char *ext=(const char*)glGetString(GL_EXTENSIONS);
        pbo_ok=extension(ext,"GL_ARB_pixel_buffer_object") || extension(ext,"GL_EXT_pixel_buffer_object");
    }
}
static void quoted(FILE *f,const char *s) {
    fputc('"',f);
    for(const unsigned char *p=(const unsigned char*)s;*p;p++) {
        if(*p=='"'||*p=='\\') fprintf(f,"\\%c",*p);
        else if(*p<32 || *p>126) fprintf(f,"\\u%04x",*p);
        else fputc(*p,f);
    } fputc('"',f);
}
static int syncdir(void) {
    int d=open(session,O_RDONLY|O_DIRECTORY|O_CLOEXEC); if(d<0) return -1;
    int e=fsync(d); if(close(d)<0) e=-1; return e;
}
static int finish(FILE *f,const char *tmp,const char *dst) {
    int bad=ferror(f); if(fflush(f)) bad=1; if(fsync(fileno(f))) bad=1;
    if(fclose(f)) bad=1;
    if(bad || rename(tmp,dst)) { unlink(tmp); return -1; }
    return syncdir();
}
static int luminance(GLint f);
static int luminance_alpha(GLint f);
static void record(unsigned id,const char *status,GLuint texture,GLint internal,
                   GLsizei w,GLsizei h,GLenum fmt,GLenum type,GLint unpack,
                   GLenum upload_error,GLenum capture_error,uint64_t bytes) {
    char path[4352],tmp[4352],raw[64];
    snprintf(path,sizeof path,"%s/event-%06u.json",session,id);
    snprintf(tmp,sizeof tmp,"%s/event-%06u.json.tmp",session,id);
    snprintf(raw,sizeof raw,"event-%06u.rgba",id);
    FILE *f=fopen(tmp,"wx"); if(!f) {perror("texcap: metadata open");return;}
    fprintf(f,"{\"event\":%u,\"pid\":%ld,\"unix_time\":%lld,\"status\":",id,(long)getpid(),(long long)time(NULL));
    quoted(f,status);
    fprintf(f,",\"texture\":%u,\"width\":%d,\"height\":%d,\"internal_format\":%d,\"upload_format\":%u,\"upload_type\":%u,\"unpack_buffer\":%d,\"upload_error\":%u,\"capture_error\":%u,\"bytes\":%llu,\"raw_file\":",texture,w,h,internal,fmt,type,unpack,upload_error,capture_error,(unsigned long long)bytes);
    if(bytes) quoted(f,raw); else fputs("null",f);
    fputs(",\"channel_expansion\":",f);quoted(f,luminance(internal)?"luminance: R replicated to RGB; alpha=255":(luminance_alpha(internal)?"luminance-alpha: R replicated to RGB; readback alpha retained":"none"));
    fputs(",\"pixel_format\":\"RGBA8\",\"row_order\":\"OpenGL row zero first (t=0); no flip\",\"provenance\":\"glGetTexImage after glTexImage2D; no artwork file reads\",\"gl_version\":",f); quoted(f,version);
    fputs(",\"vendor\":",f);quoted(f,vendor);fputs(",\"renderer\":",f);quoted(f,renderer);fputs("}\n",f);
    if(finish(f,tmp,path)) perror("texcap: metadata commit failed");
}
static int luminance(GLint f) { return f==GL_LUMINANCE || f==GL_LUMINANCE8; }
static int luminance_alpha(GLint f) { return f==GL_LUMINANCE_ALPHA || f==GL_LUMINANCE8_ALPHA8; }
static int color(GLint f) {
    return luminance(f) || luminance_alpha(f) || f==3 || f==4 || f==GL_RGB || f==GL_RGBA || f==GL_RGB8 ||
           f==GL_RGBA8 || f==GL_SRGB || f==GL_SRGB8 || f==GL_SRGB_ALPHA || f==GL_SRGB8_ALPHA8;
}
void glTexImage2D(GLenum target,GLint level,GLint internal,GLsizei w,GLsizei h,
                  GLint border,GLenum fmt,GLenum type,const void *pixels) {
    pthread_once(&once,resolve);
    if(inside || !enabled) {real_upload(target,level,internal,w,h,border,fmt,type,pixels);return;}
    pthread_mutex_lock(&lock);
    GLXContext ctx=glXGetCurrentContext();
    if(!owner && ctx) {owner=ctx;owner_thread=pthread_self();}
    if(!is_owner()) {
        fputs("texcap: skipped non-owner context/thread\n",stderr);
        pthread_mutex_unlock(&lock);real_upload(target,level,internal,w,h,border,fmt,type,pixels);return;
    }
    inside=1; drain(1);
    real_upload(target,level,internal,w,h,border,fmt,type,pixels);
    GLenum ue=drain(1),ce=0;
    if(seq>=MAX_EVENTS) {inside=0;pthread_mutex_unlock(&lock);return;}
    unsigned id=++seq; const char *status="skipped_scope";
    GLint unpack=0,tex=0,rw=0,rh=0,pack=0;
    uint64_t bytes=0; unsigned char *data=NULL;
    if(id==1) context_info();
    if(target!=GL_TEXTURE_2D || level!=0 || border!=0 || !color(internal) ||
       type!=GL_UNSIGNED_BYTE || !(fmt==GL_RGB||fmt==GL_RGBA||fmt==GL_BGR||fmt==GL_BGRA||fmt==GL_LUMINANCE||fmt==GL_LUMINANCE_ALPHA)) goto done;
    if(ue) {status="failed_upload";goto done;}
    if(pbo_ok) glGetIntegerv(GL_PIXEL_UNPACK_BUFFER_BINDING,&unpack);
    if(!pixels && !unpack) {status="skipped_null_allocation";goto done;}
    if(w<=0||h<=0||w>8192||h>8192) {status="skipped_bounds";goto done;}
    bytes=(uint64_t)w*(uint64_t)h*4;
    if(bytes>MAX_IMAGE || bytes>MAX_TOTAL-total) {status="skipped_budget";bytes=0;goto done;}
    glGetIntegerv(GL_TEXTURE_BINDING_2D,&tex);
    glGetTexLevelParameteriv(target,0,GL_TEXTURE_WIDTH,&rw);
    glGetTexLevelParameteriv(target,0,GL_TEXTURE_HEIGHT,&rh);
    ce=drain(0);
    if(ce||rw!=w||rh!=h) {status="failed_dimensions";bytes=0;goto done;}
    data=malloc((size_t)bytes);if(!data){status="failed_allocation";bytes=0;goto done;}
    GLenum names[]={GL_PACK_ALIGNMENT,GL_PACK_ROW_LENGTH,GL_PACK_SKIP_PIXELS,GL_PACK_SKIP_ROWS};
    GLint saved[4];
    for(int i=0;i<4;i++) glGetIntegerv(names[i],&saved[i]);
    if(pbo_ok) glGetIntegerv(GL_PIXEL_PACK_BUFFER_BINDING,&pack);
    ce=drain(0);if(ce){status="failed_pack_query";bytes=0;goto done;}
    for(int i=0;i<4;i++) glPixelStorei(names[i],i?0:1);
    if(pbo_ok && pack) glBindBuffer(GL_PIXEL_PACK_BUFFER,0);
    glGetTexImage(target,0,GL_RGBA,GL_UNSIGNED_BYTE,data);
    ce=drain(0);
    for(int i=0;i<4;i++) glPixelStorei(names[i],saved[i]);
    if(pbo_ok && pack) glBindBuffer(GL_PIXEL_PACK_BUFFER,(GLuint)pack);
    GLenum restore=drain(0);if(!ce) ce=restore;
    if(ce){status="failed_readback_or_restore";bytes=0;goto done;}
    /* Legacy luminance readback can expose R/0/0/A; export sampling-equivalent L/L/L/A. */
    if(luminance(internal) || luminance_alpha(internal)) {
        for(uint64_t i=0;i<bytes;i+=4) {
            data[i+1]=data[i+2]=data[i];
            if(luminance(internal)) data[i+3]=255;
        }
    }
    char path[4352],tmp[4352];
    snprintf(path,sizeof path,"%s/event-%06u.rgba",session,id);
    snprintf(tmp,sizeof tmp,"%s/event-%06u.rgba.tmp",session,id);
    FILE *f=fopen(tmp,"wx");
    if(!f){status="failed_raw_open";bytes=0;goto done;}
    total+=bytes; /* Charge attempted writes, including any failed-commit orphan. */
    int short_write=fwrite(data,1,(size_t)bytes,f)!=(size_t)bytes;
    if(short_write){fclose(f);unlink(tmp);status="failed_raw_write";bytes=0;goto done;}
    if(finish(f,tmp,path)){status="failed_raw_commit";bytes=0;goto done;}
    status="captured";
 done:
    free(data); {GLenum e=drain(0);if(!ce)ce=e;}
    record(id,status,(GLuint)tex,internal,w,h,fmt,type,unpack,ue,ce,bytes);
    if(ce || !strncmp(status,"failed",6)) fprintf(stderr,"texcap: event=%u %s error=%u\n",id,status,ce);
    if(id==MAX_EVENTS) fputs("texcap: event limit reached; further uploads forwarded without capture\n",stderr);
    inside=0;pthread_mutex_unlock(&lock);
}
static void *hook(const char *n,void *real) {
    if(!n||!real)return real;
    if(!strcmp(n,"glTexImage2D"))return (void*)glTexImage2D;
    if(!strcmp(n,"glGetError"))return (void*)glGetError;
    return real;
}
void *SDL_GL_GetProcAddress(const char *n) {
    void *(*fn)(const char*);*(void**)(&fn)=dlsym(RTLD_NEXT,"SDL_GL_GetProcAddress");
    return fn?hook(n,fn(n)):NULL;
}
__GLXextFuncPtr glXGetProcAddressARB(const GLubyte *n) {
    __GLXextFuncPtr (*fn)(const GLubyte*);*(void**)(&fn)=dlsym(RTLD_NEXT,"glXGetProcAddressARB");
    return fn?(__GLXextFuncPtr)hook((const char*)n,(void*)fn(n)):NULL;
}
__GLXextFuncPtr glXGetProcAddress(const GLubyte *n) {
    __GLXextFuncPtr (*fn)(const GLubyte*);*(void**)(&fn)=dlsym(RTLD_NEXT,"glXGetProcAddress");
    return fn?(__GLXextFuncPtr)hook((const char*)n,(void*)fn(n)):NULL;
}

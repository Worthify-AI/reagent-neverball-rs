/* SYNTHETIC pixels authored for this test; never game artwork. */
#define GL_GLEXT_PROTOTYPES
#include <SDL2/SDL.h>
#include <GL/gl.h>
#include <GL/glext.h>
#include <assert.h>
#include <stdio.h>
#include <string.h>
int main(void) {
    assert(SDL_Init(SDL_INIT_VIDEO)==0);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION,2);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION,1);
    SDL_Window *win=SDL_CreateWindow("Synthetic capture test",0,0,16,16,SDL_WINDOW_OPENGL|SDL_WINDOW_HIDDEN);
    if(!win){fprintf(stderr,"SDL: %s\n",SDL_GetError());return 1;}
    SDL_GLContext ctx=SDL_GL_CreateContext(win);assert(ctx);
    const unsigned char rgba[16]={255,0,0,0, 0,255,0,64, 0,0,255,128, 123,45,67,255};
    const unsigned char rgb[12]={255,0,0, 0,255,0, 0,0,255, 123,45,67};
    GLuint tex,pack,unpack; glGenTextures(1,&tex);glBindTexture(GL_TEXTURE_2D,tex);
    glGenBuffers(1,&pack);glBindBuffer(GL_PIXEL_PACK_BUFFER,pack);glBufferData(GL_PIXEL_PACK_BUFFER,256,NULL,GL_STREAM_READ);
    GLenum names[]={GL_PACK_ALIGNMENT,GL_PACK_ROW_LENGTH,GL_PACK_SKIP_PIXELS,GL_PACK_SKIP_ROWS};
    GLint values[]={8,9,1,2};for(int i=0;i<4;i++)glPixelStorei(names[i],values[i]);
    glPixelStorei(GL_UNPACK_ALIGNMENT,1);
    assert(glGetError()==GL_NO_ERROR);
    glEnable(0xdeadbeef); /* Must survive observer work for the app to retrieve. */
    glTexImage2D(GL_TEXTURE_2D,0,GL_RGBA8,2,2,0,GL_RGBA,GL_UNSIGNED_BYTE,rgba);
    assert(glGetError()==GL_INVALID_ENUM);assert(glGetError()==GL_NO_ERROR);
    GLint got;for(int i=0;i<4;i++){glGetIntegerv(names[i],&got);assert(got==values[i]);}
    glGetIntegerv(GL_PIXEL_PACK_BUFFER_BINDING,&got);assert(got==(GLint)pack);
    typedef void (*Upload)(GLenum,GLint,GLint,GLsizei,GLsizei,GLint,GLenum,GLenum,const void*);
    Upload upload=(Upload)SDL_GL_GetProcAddress("glTexImage2D");assert(upload);
    upload(GL_TEXTURE_2D,0,GL_RGB8,2,2,0,GL_RGB,GL_UNSIGNED_BYTE,rgb);
    assert(glGetError()==GL_NO_ERROR);
    glTexImage2D(GL_TEXTURE_2D,0,GL_RGBA8,2,2,0,GL_RGBA,GL_UNSIGNED_BYTE,NULL);
    assert(glGetError()==GL_NO_ERROR);
    glGenBuffers(1,&unpack);glBindBuffer(GL_PIXEL_UNPACK_BUFFER,unpack);
    glBufferData(GL_PIXEL_UNPACK_BUFFER,16,rgba,GL_STREAM_DRAW);
    glTexImage2D(GL_TEXTURE_2D,0,GL_RGBA8,2,2,0,GL_RGBA,GL_UNSIGNED_BYTE,(void*)0);
    assert(glGetError()==GL_NO_ERROR);
    for(int i=0;i<4;i++){glGetIntegerv(names[i],&got);assert(got==values[i]);}
    glGetIntegerv(GL_PIXEL_PACK_BUFFER_BINDING,&got);assert(got==(GLint)pack);
    glGetIntegerv(GL_PIXEL_UNPACK_BUFFER_BINDING,&got);assert(got==(GLint)unpack);
    glBindBuffer(GL_PIXEL_UNPACK_BUFFER,0);
    const unsigned char lum[4]={0,64,128,255};
    const unsigned char la[8]={17,0,63,64,129,128,251,255};
    GLint internals[4]={GL_LUMINANCE,GL_LUMINANCE8,GL_LUMINANCE_ALPHA,GL_LUMINANCE8_ALPHA8};
    for(int j=0;j<4;j++) {
        glTexImage2D(GL_TEXTURE_2D,0,internals[j],2,2,0,j<2?GL_LUMINANCE:GL_LUMINANCE_ALPHA,GL_UNSIGNED_BYTE,j<2?lum:la);
        assert(glGetError()==GL_NO_ERROR);
        for(int i=0;i<4;i++){glGetIntegerv(names[i],&got);assert(got==values[i]);}
        glGetIntegerv(GL_PIXEL_PACK_BUFFER_BINDING,&got);assert(got==(GLint)pack);
        glGetIntegerv(GL_PIXEL_UNPACK_BUFFER_BINDING,&got);assert(got==0);
    }
    glTexImage2D(GL_TEXTURE_2D,0,GL_RGBA8,-1,2,0,GL_RGBA,GL_UNSIGNED_BYTE,(void*)0);
    assert(glGetError()==GL_INVALID_VALUE);assert(glGetError()==GL_NO_ERROR);
    glBindBuffer(GL_PIXEL_UNPACK_BUFFER,0);glBindBuffer(GL_PIXEL_PACK_BUFFER,0);
    glDeleteBuffers(1,&pack);glDeleteBuffers(1,&unpack);glDeleteTextures(1,&tex);
    SDL_GL_DeleteContext(ctx);SDL_DestroyWindow(win);SDL_Quit();
    puts("SYNTHETIC FIXTURE PASS: GL errors, pack state, PBO bindings; run convert.py --fixture for exact pixels.");
    return 0;
}

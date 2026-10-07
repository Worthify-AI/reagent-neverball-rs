/* SPDX-License-Identifier: GPL-3.0-or-later
 * Worthify reference harness. SDL API interposition only; no Neverball source used.
 * Synthetic key events, a 60 Hz logical clock, and real OpenGL frame capture.
 */
#define _GNU_SOURCE
#include <SDL2/SDL.h>
#include <GL/gl.h>
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
static unsigned frame, ticks, ticks_step;
static FILE *video, *input_log;
static int event_index, max_frames=900, initialized;
typedef struct { unsigned at; int key, down; } Input;
static Input script[256];static int script_count;
static void init(void) {
 if(initialized) return;
 initialized=1;signal(SIGPIPE,SIG_IGN);
 const char *limit=getenv("WORTHIFY_FRAMES");if(limit)max_frames=atoi(limit);
 const char *path=getenv("WORTHIFY_INPUTS");if(path){FILE*f=fopen(path,"r");if(f){while(script_count<256&&fscanf(f,"%u %d %d",&script[script_count].at,&script[script_count].key,&script[script_count].down)==3)script_count++;fclose(f);}}
 input_log=fopen("/worthify/work/reference/recordings/inputs-observed.csv","w");if(input_log)fprintf(input_log,"frame,logical_ms,key,down\n");
 video=popen("ffmpeg -loglevel warning -y -f rawvideo -pixel_format rgb24 -video_size 800x600 -framerate 30 -i pipe:0 -vf vflip -c:v libx264 -crf 22 -pix_fmt yuv420p /worthify/work/reference/recordings/reference-capture.mp4 2>/worthify/work/reference/recordings/encoder.log","w");
}
Uint32 SDL_GetTicks(void) { return ticks; }
int SDL_PollEvent(SDL_Event*e) {
 init();static int(*real)(SDL_Event*);if(!real)real=dlsym(RTLD_NEXT,"SDL_PollEvent");
 while(real(e)) { if(e->type<SDL_KEYDOWN||e->type>SDL_MOUSEWHEEL)return 1; }
 if(frame >= (unsigned)max_frames){memset(e,0,sizeof(*e));e->type=SDL_QUIT;return 1;}
 if(event_index<script_count&&script[event_index].at<=frame){Input x=script[event_index++];memset(e,0,sizeof(*e));e->type=x.down?SDL_KEYDOWN:SDL_KEYUP;e->key.state=x.down?SDL_PRESSED:SDL_RELEASED;e->key.keysym.sym=x.key;e->key.keysym.scancode=SDL_GetScancodeFromKey(x.key);if(input_log){fprintf(input_log,"%u,%u,%d,%d\n",frame,ticks,x.key,x.down);fflush(input_log);}return 1;}
 ticks_step++;ticks=(ticks_step*1000)/60;return 0;
}
void SDL_GL_SwapWindow(SDL_Window*w) {
 init();static void(*real)(SDL_Window*);if(!real)real=dlsym(RTLD_NEXT,"SDL_GL_SwapWindow");
 if(video&&frame%2==0){static unsigned char pixels[800*600*3];glPixelStorei(GL_PACK_ALIGNMENT,1);glReadPixels(0,0,800,600,GL_RGB,GL_UNSIGNED_BYTE,pixels);if(fwrite(pixels,1,sizeof(pixels),video)!=sizeof(pixels)){pclose(video);video=NULL;}}
 fflush(NULL);real(w);frame++;
}
__attribute__((destructor)) static void done(void){if(video)pclose(video);if(input_log)fclose(input_log);}

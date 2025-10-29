#ifndef _ARIA_JNI_H
#define _ARIA_JNI_H

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================
   Basic JNI type definitions
   ============================================================ */

typedef unsigned char jboolean;
typedef signed char jbyte;
typedef unsigned short jchar;
typedef short jshort;
typedef int jint;
typedef long long jlong;
typedef float jfloat;
typedef double jdouble;

typedef void* jobject;
typedef jobject jclass;
typedef jobject jstring;
typedef jobject jarray;
typedef jobject jthrowable;
typedef jobject jweak;
typedef jobject jref;

#define JNI_FALSE 0
#define JNI_TRUE 1

/* ============================================================
   Java VM and JNIEnv forward declarations
   ============================================================ */
struct JNINativeInterface_;
struct JavaVMInterface_;

typedef const struct JNINativeInterface_* JNIEnv;
typedef const struct JavaVMInterface_* JavaVM;

/* ============================================================
   JNI function modifiers
   ============================================================ */
#ifdef _WIN32
    #define JNIEXPORT __declspec(dllexport)
    #define JNICALL __stdcall
#else
    #define JNIEXPORT __attribute__((visibility("default")))
    #define JNICALL
#endif

/* ============================================================
   JNI version constants
   ============================================================ */
#define JNI_VERSION_1_1 0x00010001
#define JNI_VERSION_1_2 0x00010002
#define JNI_VERSION_1_4 0x00010004
#define JNI_VERSION_1_6 0x00010006
#define JNI_VERSION_1_8 0x00010008
#define JNI_VERSION_9   0x00090000
#define JNI_VERSION_10  0x000a0000
#define JNI_VERSION_11  0x000b0000
#define JNI_VERSION_17  0x00110000  /* AriaJDK uses Java 17 baseline */

/* ============================================================
   Minimal JNINativeInterface layout
   ============================================================ */
struct JNINativeInterface_ {
    void* reserved0;
    void* reserved1;
    void* reserved2;
    void* reserved3;

    jint (*GetVersion)(JNIEnv* env);

    jclass (*FindClass)(JNIEnv* env, const char* name);
    jmethodID (*GetMethodID)(JNIEnv* env, jclass clazz, const char* name, const char* sig);
    jobject (*NewObject)(JNIEnv* env, jclass clazz, jmethodID methodID, ...);
    void (*CallVoidMethod)(JNIEnv* env, jobject obj, jmethodID methodID, ...);

    const char* (*GetStringUTFChars)(JNIEnv* env, jstring string, jboolean* isCopy);
    void (*ReleaseStringUTFChars)(JNIEnv* env, jstring string, const char* utf);

    void (*ExceptionDescribe)(JNIEnv* env);
    void (*ExceptionClear)(JNIEnv* env);

    void (*FatalError)(JNIEnv* env, const char* msg);
};

/* ============================================================
   JavaVM interface (simplified)
   ============================================================ */
struct JavaVMInterface_ {
    jint (*DestroyJavaVM)(JavaVM* vm);
    jint (*AttachCurrentThread)(JavaVM* vm, void** penv, void* args);
    jint (*DetachCurrentThread)(JavaVM* vm);
    jint (*GetEnv)(JavaVM* vm, void** penv, jint version);
};

/* ============================================================
   Helper macros for native declarations
   ============================================================ */
#define JNI_FUNC(name) JNIEXPORT void JNICALL Java_##name

#ifdef __cplusplus
}
#endif

#endif /* _ARIA_JNI_H */

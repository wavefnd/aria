#ifndef _ARIA_JNI_MD_H_
#define _ARIA_JNI_MD_H_

/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * AriaJDK machine-dependent JNI declarations.
 */

#ifndef __has_attribute
#define __has_attribute(x) 0
#endif

#ifndef JNIEXPORT
#if defined(_WIN32) || defined(__CYGWIN__)
#define JNIEXPORT __declspec(dllexport)
#else
#if (defined(__GNUC__) && (__GNUC__ > 4 || (__GNUC__ == 4 && __GNUC_MINOR__ > 2))) || __has_attribute(visibility)
#define JNIEXPORT __attribute__((visibility("default")))
#else
#define JNIEXPORT
#endif
#endif
#endif

#ifndef JNIIMPORT
#if defined(_WIN32) || defined(__CYGWIN__)
#define JNIIMPORT __declspec(dllimport)
#else
#if (defined(__GNUC__) && (__GNUC__ > 4 || (__GNUC__ == 4 && __GNUC_MINOR__ > 2))) || __has_attribute(visibility)
#define JNIIMPORT __attribute__((visibility("default")))
#else
#define JNIIMPORT
#endif
#endif
#endif

#ifndef JNICALL
#if defined(_WIN32) && !defined(_WIN64)
#define JNICALL __stdcall
#else
#define JNICALL
#endif
#endif

typedef int jint;
typedef signed char jbyte;

#if defined(_WIN64) || defined(__LP64__) || defined(_LP64)
typedef long jlong;
#else
typedef long long jlong;
#endif

#endif /* _ARIA_JNI_MD_H_ */

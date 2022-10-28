# NickAc's Minecraft Skin Renderer

This is my attempt at making a Minecraft skin renderer.

However, do keep in mind that this doesn't actually render anything, instead it uses some pre-rendered UV maps to then
find-and replace the contents with the actual skin.

The input must be a 64x64 skin.

## Modules

### nmsr-lib

This is the core library, which contains the actual skin renderer.

### nmsr-jni

This is the JNI library, which contains the native code for invoking the skin renderer from the JVM.

### NMSRaaS - NickAc's Minecraft Skin Renderer as a Service

This is the web service, which allows to render skins with a simple HTTP request.
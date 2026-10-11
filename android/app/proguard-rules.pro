# SPDX-License-Identifier: GPL-3.0-or-later
# JNA dispatches UniFFI methods and Structure fields through reflection.
-keep class com.sun.jna.** { *; }
# UniFFI 0.32 uses an object with Native.register plus callbacks/vtable structures.
# Keep its generated API intact across R8 optimization and reflection.
-keep class com.codedbydan.momentum.core.** { *; }
-keepclassmembers class * implements com.sun.jna.Callback { public *; }
-dontwarn java.awt.**

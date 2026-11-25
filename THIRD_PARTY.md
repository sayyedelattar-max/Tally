# Third-party software

## libmpv (high-quality player)
The Windows installer ships `libmpv-2.dll`, an **LGPL** build of [mpv](https://mpv.io) from
<https://github.com/zhongfly/mpv-winbuild>. Tally loads it at run time as a separate dynamic library and
does not modify it. mpv and FFmpeg are licensed under the LGPL v2.1 or later; their source and licence texts
are available from the projects above. You may replace the DLL with another LGPL libmpv build of the same
major version.

## mpegts.js
Built-in player for MPEG-TS streams (Apache-2.0).

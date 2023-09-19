# Building under the W.S.L.

Users of Microsoft Windows 10 and 11 may wish to build their Bevy projects under the *Windows Subsystem for Linux*, using Linux-based tooling, for a variety of reasons and there are several approaches to doing so.

The first set of approaches involve cross-compilation: building Windows artefacts from the Linux developer environment. These artefacts will be Windows executables and will run natively on the Windows host or may even be deployed to users who run Windows, given some considerations. As such, they will be able to present first-class desktop windows and have native access to the GPU, sound and other relevant hardware.

The second set of approaches exploit the capability of recent WSL 2 versions to run GUI applications – available under Windows 11 *and* under Windows 10, given updates to the WSL 2. These approaches produce Linux executables that will not likely be deployable but will run, directly, from the WSL's virtualised filesystem. Some configuration is required and other considerations should be addressed.

## Cross-Compilation

Cross-compiling applications for Windows, under the WSL, is an appealing option and is no different from [cross-compilation for Windows](https://bevy-cheatbook.github.io/setup/cross/linux-windows.html) under *any* Linux environment – at least at build time. A brief summary of cross compilation approaches follows in sub-sections.

While cross-compilation approaches may produce executables that are perfectly sound, these do not expect to be run from the virtualised Linux file system within the WSL. Developers seeking to run built apps will need to consider marshalling the executables and assets to the host's NTFS file system – mounted under Linux as `/mnt/c/…` et al. – and executing their apps from there.

### Targetting the Microsoft Visual C++ Runtime

Bevy applications may be compiled under Linux to target the Microsoft Visual C++ Runtime – the `x86_64-pc-windows-msvc` Rust target – and entails some setup but the result is the closest to the native Windows experience, at run-time and also where deployment is concerned.

Building for the `x86_64-pc-windows-msvc` target may be achieved by using LLVM tooling and clang, providing the Microsoft Windows SDK official headers and libraries which may be fetched from Microsoft, themselves, with tools such as [`xwin`](https://github.com/Jake-Shadle/xwin/). `xwin` may be installed with cargo.

The deployment of Windows applications that target the MSVC runtime is identical to that of applications built natively with Microsoft's own compilers and tooling.

### Targetting MinGW

Bevy applications may be compiled under Linux to target [MinGW](https://sourceforge.net/projects/mingw/) – i.e. the Rust target: `x86_64-pc-windows-gnu`. Given that the tool-chain and target is installed under the Linux environment, this is as trivial as:

```sh
cargo build --target x86_64-pc-windows-gnu
```

Support for the `x86_64-pc-windows-gnu` may even be installed with [rustup](https://rustup.rs/) and is further documented within the [unofficial Bevy cheat-book](https://bevy-cheatbook.github.io/setup/cross/linux-windows.html).

Targetting MinGW carries deployment concerns beyond those of applications targetting the MSVC runtime.

## Linux GUI Application

Recent releases of the WSL 2 include [**WSLg** – *Windows Subsystem for Linux GUI*](https://github.com/microsoft/wslg) – on both Windows 11 *and* Windows 10 and support the running of GUI applications. No manual installation is necessary: support for WSLg may be determined by executing `wsl.exe --version` on the Windows host; for example:

```
C:\...> wsl.exe --version
WSL version: 1.2.0.0
Kernel version: 5.15.90.1
WSLg version: 1.0.51
MSRDC version: 1.2.3770
Direct3D version: 1.608.2-61064218
DXCore version: 10.0.25131.1002-220531-1700.rs-onecore-base2-hyp
Windows version: 10.0.19045.2846
```

(Additionally, see `/mnt/wslg/versions.txt` within the Linux environment.)

However, [Vulkan support is *not* stable under the WSL 2 (microsoft/WSL#7790)](https://github.com/microsoft/WSL/issues/7790) (and [microsoft/wslg#40](https://github.com/microsoft/wslg/issues/40)) and this presents a problem for Bevy applications.

The [prerequisites required to build Bevy applications](./linux_dependencies.md) and those required to run them under the WSL are distribution specific. Under *Ubuntu*, the default distribution for the WSL, graphical applications may be run without much additional setup and with dependencies sourced via `apt`, using the packages `libvulkan1` and `mesa-vulkan-drivers`, although the app will use the CPU (via `llvmpipe`) and not the GPU for rendering at run-time.

See the following links – all discussing Ubuntu – for further insight:

- [Run Linux GUI apps on the WSL — Microsoft](https://learn.microsoft.com/en-us/windows/wsl/tutorials/gui-apps)
- [Install and run GUI apps — WSLg, Microsoft](https://github.com/microsoft/wslg#install-and-run-gui-apps)
- [Discussions in #5040](https://github.com/bevyengine/bevy/pull/5040#issuecomment-1412986908)
- [Running Graphical Applications on the WSL — Ubuntu](https://wiki.ubuntu.com/WSL#Running_Graphical_Applications)

### Microsoft "Dozen" / "Dzn"

Experimental code named "Dozen" or "Dzn", from Microsoft, implements *Vulkan* as a layer on top of *Direct3D 12*. It exists as a driver within the *Mesa* 3D graphics library and is [reported to achieve 98.5% coverage of Vulkan 1.0](https://www.phoronix.com/news/MS-Dozen-98.5p-Vulkan-1.0).

Users who are prepared to build *Mesa* from source may investigate this as an option for GPU accelerated rendering under the WSL and, in the future, it is possible that this work will offer stable Vulkan support.

- This is also [discussed in #5040](https://github.com/bevyengine/bevy/pull/5040#issuecomment-1494706996).

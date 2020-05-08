#!/usr/bin/env bash

# This shell script strips out the -landroid that is passed by default by rustc to
# the linker on aarch64-linux-android, and adds some entries to the ld search path.

set -o errexit
set -o nounset
set -o pipefail

TARGET=${TARGET:-"aarch64-linux-android"}
LD=${LD:-"${MAGICLEAP_SDK}/tools/toolchains/bin/${TARGET}-ld"}
LDFLAGS=${LDFLAGS:-"--sysroot=${MAGICLEAP_SDK}/lumin -L${MAGICLEAP_SDK}/lumin/usr/lib -L${MAGICLEAP_SDK}/tools/toolchains/lib/gcc/${TARGET}/4.9.x ${MAGICLEAP_SDK}/lumin/usr/lib/crtbegin_so.o"}

# Remove the -landroid flag, grr
ARGS=("$@")
ARGS=${ARGS[@]/-landroid}

${LD} ${LDFLAGS} ${ARGS}

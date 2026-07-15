#!/usr/bin/env bash
# Point wgpu at a software GPU adapter so egui_kittest can render frames headlessly
# (for the debug driver's `screenshot` command and the snapshot tests).
#
# Usage:
#   source scripts/headless-gpu.sh          # export the vars into your shell
#   scripts/headless-gpu.sh cargo test ...   # or run a command with them set
#
# Interaction, `tree`, and `state` need NO GPU — only screenshots/snapshots do.
#
# Fallback chain (first that exists wins):
#   1. SwiftShader Vulkan ICD bundled with the pre-installed Chromium.
#   2. lavapipe (Mesa) Vulkan ICD, if `apt-get install mesa-vulkan-drivers` was run.
#   3. wgpu's GL backend on llvmpipe (software OpenGL).

_swiftshader_icd() {
  # The chromium build dir is versioned; pick whichever is present.
  local f
  for f in /opt/pw-browsers/chromium-*/chrome-linux/vk_swiftshader_icd.json; do
    [ -f "$f" ] && { echo "$f"; return 0; }
  done
  return 1
}

if icd="$(_swiftshader_icd)"; then
  export VK_ICD_FILENAMES="$icd"
  export VK_DRIVER_FILES="$icd"   # newer Vulkan loader env name
  echo "headless-gpu: using SwiftShader Vulkan ICD at $icd" >&2
elif [ -f /usr/share/vulkan/icd.d/lvp_icd.x86_64.json ]; then
  export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json
  export VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json
  echo "headless-gpu: using lavapipe (Mesa) Vulkan ICD" >&2
else
  export WGPU_BACKEND=gl
  export LIBGL_ALWAYS_SOFTWARE=1
  echo "headless-gpu: no Vulkan ICD found; falling back to software GL (WGPU_BACKEND=gl)" >&2
  echo "headless-gpu: for Vulkan, run: apt-get install -y mesa-vulkan-drivers libvulkan1" >&2
fi

# If invoked with a command (not sourced), run it with the vars set.
if [ "${BASH_SOURCE[0]}" = "$0" ] && [ "$#" -gt 0 ]; then
  exec "$@"
fi

#!/bin/sh
# Downloads three.js (MIT license) into ui/vendor/ for the 3D background. Run once from the repo root.
set -e
mkdir -p ui/vendor
curl -fL "https://cdn.jsdelivr.net/npm/three@0.170.0/build/three.module.min.js" -o ui/vendor/three.module.min.js
echo "Saved three.js r170 to ui/vendor/three.module.min.js"

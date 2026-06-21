     STDIN
   1 #!/bin/bash
   2 set -e
   3 
   4 # 用于发布时更新 Package.swift 的脚本
   5 # 使用方法: ./scripts/update-package-for-release.sh <version> <checksum>
   6 
   7 VERSION=$1
   8 CHECKSUM=$2
   9 
  10 if [ -z "$VERSION" ] || [ -z "$CHECKSUM" ]; then
  11     echo "Usage: $0 <version> <checksum>"
  12     echo "Example: $0 1.0.0 abc123..."
  13     exit 1
  14 fi
  15 
  16 REPO_URL="https://github.com/ending0421/Scribe/releases/download"
  17 FRAMEWORK_URL="${REPO_URL}/v${VERSION}/Scribe.xcframework.zip"
  18 
  19 echo "Updating Package.swift for release v${VERSION}"
  20 echo "Framework URL: ${FRAMEWORK_URL}"
  21 echo "Checksum: ${CHECKSUM}"
  22 
  23 cat > Package.swift << PACKAGE_EOF
  24 // swift-tools-version:5.9
  25 import PackageDescription
  26 
  27 let package = Package(
  28     name: "Scribe",
  29     platforms: [
  30         .iOS(.v13),
  31         .macOS(.v10_15)
  32     ],
  33     products: [
  34         .library(
  35             name: "Scribe",
  36             targets: ["Scribe"]
  37         ),
  38     ],
  39     targets: [
  40         .target(
  41             name: "Scribe",
  42             dependencies: ["ScribeFFI"],
  43             path: "Sources/Scribe"
  44         ),
  45         .binaryTarget(
  46             name: "ScribeFFI",
  47             url: "${FRAMEWORK_URL}",
  48             checksum: "${CHECKSUM}"
  49         ),
  50     ]
  51 )
  52 PACKAGE_EOF
  53 
  54 echo "✅ Package.swift updated successfully"

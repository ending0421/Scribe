     STDIN
   1 // swift-tools-version:5.9
   2 import PackageDescription
   3 
   4 let package = Package(
   5     name: "Scribe",
   6     platforms: [
   7         .iOS(.v13),
   8         .macOS(.v10_15)
   9     ],
  10     products: [
  11         .library(
  12             name: "Scribe",
  13             targets: ["Scribe"]
  14         ),
  15     ],
  16     targets: [
  17         .target(
  18             name: "Scribe",
  19             dependencies: ["ScribeFFI"],
  20             path: "Sources/Scribe"
  21         ),
  22         // 本地开发时使用本地路径
  23         // 发布后，在 GitHub 上会指向 Release 的 URL
  24         .binaryTarget(
  25             name: "ScribeFFI",
  26             path: "Scribe.xcframework"
  27         ),
  28     ]
  29 )

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
  22         .binaryTarget(
  23             name: "ScribeFFI",
  24             path: "Scribe.xcframework"
  25         ),
  26     ]
  27 )

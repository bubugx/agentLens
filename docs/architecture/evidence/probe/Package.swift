// swift-tools-version: 5.10
import PackageDescription
let package = Package(name: "AgentLensArchitectureProbe", platforms: [.macOS(.v14)], products: [.executable(name: "Probe", targets: ["Probe"])], targets: [.systemLibrary(name: "CSQLite"), .executableTarget(name: "Probe", dependencies: ["CSQLite"])])

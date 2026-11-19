import SwiftUI
struct GlassUnavailable: View {
    var body: some View {
        if #available(macOS 26.0, *) { Text("Probe").glassEffect() }
        else { Text("Probe").background(.regularMaterial) }
    }
}

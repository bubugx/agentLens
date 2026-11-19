import SwiftUI
struct GlassGuarded: View {
    @ViewBuilder var body: some View {
        #if compiler(>=6.2) && AGENTLENS_HAS_LIQUID_GLASS
        if #available(macOS 26.0, *) { Text("Probe").glassEffect() }
        else { Text("Probe").background(.regularMaterial) }
        #else
        Text("Probe").background(.regularMaterial)
        #endif
    }
}

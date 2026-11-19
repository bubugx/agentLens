import Foundation
import SwiftUI
import AppKit
import Charts
import Observation
import CoreServices
import Darwin
import CSQLite

@Observable final class Store { var count = 0 }
struct SmokeView: View {
    @Environment(\.accessibilityReduceTransparency) var reduceTransparency
    @Environment(\.accessibilityReduceMotion) var reduceMotion
    var body: some View { NavigationSplitView { Text("AgentLens") } detail: {
        Chart { BarMark(x: .value("Sample", 1), y: .value("Bytes", 2)) }
            .background(.regularMaterial)
    } }
}
struct SmokeScene: Scene {
    var body: some Scene { MenuBarExtra("AgentLens", systemImage: "chart.bar") { SmokeView() }.menuBarExtraStyle(.window) }
}
let pid = getpid()
var proc = proc_taskinfo()
let procSize = Int32(MemoryLayout<proc_taskinfo>.stride)
let procRead = proc_pidinfo(pid, PROC_PIDTASKINFO, 0, &proc, procSize)
var bsd = proc_bsdinfo()
let bsdRead = proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &bsd, Int32(MemoryLayout<proc_bsdinfo>.stride))
var vm = vm_statistics64()
var count = mach_msg_type_number_t(MemoryLayout<vm_statistics64>.size / MemoryLayout<integer_t>.size)
let vmResult = withUnsafeMutablePointer(to: &vm) { $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) { host_statistics64(mach_host_self(), HOST_VM_INFO64, $0, &count) } }
var pressure: Int32 = 0
var pressureSize = MemoryLayout.size(ofValue: pressure)
let pressureResult = sysctlbyname("kern.memorystatus_vm_pressure_level", &pressure, &pressureSize, nil, 0)
var path = [CChar](repeating: 0, count: 4 * Int(MAXPATHLEN))
let pathSize = proc_pidpath(pid, &path, UInt32(path.count))
var db: OpaquePointer?
let sqlResult = sqlite3_open(":memory:", &db)
if let db = db { sqlite3_close(db) }
let trashAPI: (URL, AutoreleasingUnsafeMutablePointer<NSURL?>?) throws -> Void = FileManager.default.trashItem(at:resultingItemURL:)
let eventsAPI = FSEventStreamCreate
let panel = NSOpenPanel.self
let workspace = NSWorkspace.self
print("SwiftUI/AppKit/Charts/Observation/MenuBarExtra/material/accessibility compile: OK")
print("libproc self task info: \(procRead)/\(procSize), RSS \(proc.pti_resident_size)")
print("libproc BSD identity: \(bsdRead), start \(bsd.pbi_start_tvsec).\(bsd.pbi_start_tvusec)")
print("libproc executable path bytes: \(pathSize)")
print("host_statistics64 result: \(vmResult), pageSize: \(vm_kernel_page_size), freePages: \(vm.free_count)")
print("memory pressure sysctl result: \(pressureResult), value: \(pressure)")
print("SQLite in-memory open: \(sqlResult), version: \(String(cString: sqlite3_libversion()))")
print("FSEvents, NSOpenPanel, NSWorkspace and trashItem symbols: OK (no UI or trash operation performed)")

// Rasterises an SVG to square PNGs using AppKit (no third-party tools needed on macOS).
// usage: swift render-svg.swift <in.svg> <out-dir> <base-name> <size,size,...>
// writes <out-dir>/<base-name>-<size>.png for each size.

import Cocoa

let args = CommandLine.arguments
guard args.count == 5 else {
    FileHandle.standardError.write("usage: render-svg.swift <in.svg> <out-dir> <base-name> <sizes>\n".data(using: .utf8)!)
    exit(2)
}
let svgURL = URL(fileURLWithPath: args[1])
let outDir = args[2]
let base = args[3]
let sizes = args[4].split(separator: ",").compactMap { Int($0) }

guard let image = NSImage(contentsOf: svgURL) else {
    FileHandle.standardError.write("cannot load \(svgURL.path)\n".data(using: .utf8)!)
    exit(1)
}
try? FileManager.default.createDirectory(atPath: outDir, withIntermediateDirectories: true)

for size in sizes {
    guard let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
                                     bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                                     colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0) else { continue }
    rep.size = NSSize(width: size, height: size)
    NSGraphicsContext.saveGraphicsState()
    let ctx = NSGraphicsContext(bitmapImageRep: rep)
    NSGraphicsContext.current = ctx
    ctx?.imageInterpolation = .high
    image.draw(in: NSRect(x: 0, y: 0, width: size, height: size), from: .zero, operation: .copy, fraction: 1)
    NSGraphicsContext.restoreGraphicsState()
    guard let png = rep.representation(using: .png, properties: [:]) else { continue }
    let out = "\(outDir)/\(base)-\(size).png"
    try! png.write(to: URL(fileURLWithPath: out))
    print("wrote \(out)")
}

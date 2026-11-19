import Cocoa

/// A menu row with the item's text on the left and a ☆/★ button on the right.
///
/// Clicking the text fires `onClick` with the modifier flags held (plain = copy,
/// ⌥ = paste, ⌘ = remove — decided by the caller). Clicking the star toggles it in
/// place and fires `onStar`, keeping the menu open.
final class ClipRowView: NSView {
    static let rowWidth: CGFloat = 400
    static let rowHeight: CGFloat = 24

    var onClick: ((NSEvent.ModifierFlags) -> Void)?
    var onStar: (() -> Void)?

    private let label = NSTextField(labelWithString: "")
    private let starButton = NSButton(title: "☆", target: nil, action: nil)
    private var trackingArea: NSTrackingArea?

    private(set) var starred: Bool {
        didSet { applyStarLook() }
    }

    private var hovered = false {
        didSet {
            needsDisplay = true
            label.textColor = hovered ? .selectedMenuItemTextColor : .labelColor
            applyStarLook()
        }
    }

    init(title: String, starred: Bool) {
        self.starred = starred
        super.init(frame: NSRect(x: 0, y: 0, width: ClipRowView.rowWidth, height: ClipRowView.rowHeight))
        autoresizingMask = [.width]

        label.stringValue = title
        label.font = .menuFont(ofSize: 0)
        label.lineBreakMode = .byTruncatingTail
        label.cell?.truncatesLastVisibleLine = true
        label.frame = NSRect(x: 14, y: 3, width: ClipRowView.rowWidth - 14 - 40, height: 18)
        label.autoresizingMask = [.width]
        addSubview(label)

        starButton.isBordered = false
        starButton.font = .systemFont(ofSize: 15)
        starButton.target = self
        starButton.action = #selector(starTapped)
        starButton.frame = NSRect(x: ClipRowView.rowWidth - 34, y: 1, width: 26, height: 22)
        starButton.autoresizingMask = [.minXMargin]
        starButton.toolTip = "Star / unstar (starred items are never removed)"
        addSubview(starButton)

        applyStarLook()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    private func applyStarLook() {
        starButton.title = starred ? "★" : "☆"
        starButton.contentTintColor = starred
            ? .systemYellow
            : (hovered ? .selectedMenuItemTextColor : .tertiaryLabelColor)
    }

    // MARK: - Hover highlight (custom views don't get the menu's own highlight)

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let old = trackingArea { removeTrackingArea(old) }
        let area = NSTrackingArea(rect: bounds,
                                  options: [.mouseEnteredAndExited, .activeAlways, .inVisibleRect],
                                  owner: self, userInfo: nil)
        addTrackingArea(area)
        trackingArea = area
    }

    override func mouseEntered(with event: NSEvent) { hovered = true }
    override func mouseExited(with event: NSEvent) { hovered = false }

    override func draw(_ dirtyRect: NSRect) {
        guard hovered else { return }
        NSColor.selectedContentBackgroundColor.setFill()
        NSBezierPath(roundedRect: bounds.insetBy(dx: 5, dy: 0), xRadius: 5, yRadius: 5).fill()
    }

    // MARK: - Clicks

    override func mouseUp(with event: NSEvent) {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        hovered = false
        enclosingMenuItem?.menu?.cancelTracking()
        onClick?(flags)
    }

    @objc private func starTapped() {
        starred.toggle()
        onStar?()
    }
}

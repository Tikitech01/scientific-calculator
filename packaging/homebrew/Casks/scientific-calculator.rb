cask "scientific-calculator" do
  version "3.1.1"
  sha256 "01c7be0f05c84ae785e113f63c16d1767fe031f188f05c948cf553f672f377c9"

  url "https://pub-8a5cc98180e2407488e6a7c7e4d86a4b.r2.dev/scientific-calculator/3.1.1/Scientific-Calculator-Studio-Pro-3.1.1-macOS-AppleSilicon.zip"
  name "Scientific Calculator Studio Pro"
  desc "Scientific Calculator Studio Pro for engineering students"
  homepage "https://github.com/Tikitech01/scientific-calculator"

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"

  app "Scientific Calculator Studio Pro.app"

  zap trash: [
    "~/Library/Application Support/Scientific Calculator",
  ]

  caveats <<~EOS
    Scientific Calculator Studio Pro is not notarised by Apple yet. If macOS says it cannot be opened,
    open System Settings > Privacy & Security and click "Open Anyway".
  EOS
end

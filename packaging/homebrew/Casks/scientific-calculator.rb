cask "scientific-calculator" do
  version "2.4.0"
  sha256 "3e56ebdd5ac686b6bad340aae35bdc4e6482e30d7b8e4f3b1d7a64d83ac7bd13"

  url "https://github.com/Tikitech01/scientific-calculator/raw/72180f021702712b74807955c1248a86026ea4b4/downloads/Scientific-Calculator-2.4.0-macOS-AppleSilicon.zip"
  name "Scientific Calculator"
  desc "Scientific, graphing, statistics and financial calculator for engineering students"
  homepage "https://github.com/Tikitech01/scientific-calculator"

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"

  app "Scientific Calculator.app"

  zap trash: [
    "~/Library/Application Support/Scientific Calculator",
  ]

  caveats <<~EOS
    Scientific Calculator is not notarised by Apple yet. If macOS says it cannot be opened,
    open System Settings > Privacy & Security and click "Open Anyway".
  EOS
end

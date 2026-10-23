cask "scientific-calculator" do
  version "3.1.0"
  sha256 "c941baec2fbff1a7425d525033b77c2357183cd5b355894325fe7479f0025af7"

  url "https://github.com/Tikitech01/scientific-calculator/raw/710c0d88f0d2a7dcca566b4c96b8a90219987a7a/downloads/Scientific-Calculator-3.1.0-macOS-AppleSilicon.zip"
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

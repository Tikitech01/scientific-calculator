cask "scientific-calculator" do
  version "3.0.0"
  sha256 "910f56df9e06cef45cdea597df5945517869318ae0e5f9742c288568bd66dfb4"

  url "https://github.com/Tikitech01/scientific-calculator/raw/d1c674832cbb826b9ec56c3478f240d482d9394b/downloads/Scientific-Calculator-3.0.0-macOS-AppleSilicon.zip"
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

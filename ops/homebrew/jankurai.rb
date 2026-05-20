# Homebrew formula metadata for the release pipeline.
# release-publish renders this template to dist/jankurai-homebrew.rb.
class Jankurai < Formula
  desc "Audit CLI for trustworthy AI-assisted merge"
  homepage "https://github.com/neverhuman/jankurai"
  url "https://github.com/neverhuman/jankurai.git", tag: "__RELEASE_TAG__"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--path", "crates/jankurai", "--locked", "--root", prefix
  end

  test do
    system bin/"jankurai", "version"
  end
end

class Rx < Formula
  desc "A fast, unified Rust toolchain manager"
  homepage "https://github.com/iPeluwa/rx"
  license "MIT"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/iPeluwa/rx/releases/download/v#{version}/rx-aarch64-apple-darwin.tar.gz"
      sha256 "" # TODO: fill after release
    else
      url "https://github.com/iPeluwa/rx/releases/download/v#{version}/rx-x86_64-apple-darwin.tar.gz"
      sha256 "" # TODO: fill after release
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/iPeluwa/rx/releases/download/v#{version}/rx-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "" # TODO: fill after release
    else
      url "https://github.com/iPeluwa/rx/releases/download/v#{version}/rx-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "" # TODO: fill after release
    end
  end

  def install
    bin.install "rx"

    # Generate and install shell completions
    generate_completions_from_executable(bin/"rx", "completions")

    # Generate and install man page
    man1.install Utils.safe_popen_read(bin/"rx", "manpage").to_s => "rx.1"
  end

  test do
    assert_match "rx #{version}", shell_output("#{bin}/rx --version")
  end
end

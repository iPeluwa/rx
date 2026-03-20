class Rx < Formula
  desc "A fast, unified Rust toolchain manager"
  homepage "https://github.com/iPeluwa/rx"
  license "MIT"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/iPeluwa/rx/releases/download/v#{version}/rx-aarch64-apple-darwin.tar.gz"
      sha256 "1e3f01234dca401eee51bfa6c79608941c8bba77901148d2747adf287847af80"
    else
      url "https://github.com/iPeluwa/rx/releases/download/v#{version}/rx-x86_64-apple-darwin.tar.gz"
      sha256 "2804ec706e676794ee761b4575c60023fd5a6d135ddf1db5794953dd04f815bd"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/iPeluwa/rx/releases/download/v#{version}/rx-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "53c6cc2b53dac00686aa6ec5fdd82f53c3e8e689303243380efc6dc1ac0c42f2"
    else
      url "https://github.com/iPeluwa/rx/releases/download/v#{version}/rx-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "a0329f50ce7bf8d6c1f83b9716415d429a06df40ec4fa36874e55d9bb775f975"
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

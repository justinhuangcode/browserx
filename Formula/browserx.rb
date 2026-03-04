class Browserx < Formula
  desc "Extract browser cookies from any browser -- CLI for AI agents and automation"
  homepage "https://github.com/justinhuangcode/browserx"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/justinhuangcode/browserx/releases/download/v#{version}/browserx-v#{version}-x86_64-apple-darwin.tar.gz"
      # sha256 will be filled by release CI
    end

    on_arm do
      url "https://github.com/justinhuangcode/browserx/releases/download/v#{version}/browserx-v#{version}-aarch64-apple-darwin.tar.gz"
      # sha256 will be filled by release CI
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/justinhuangcode/browserx/releases/download/v#{version}/browserx-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      # sha256 will be filled by release CI
    end

    on_arm do
      url "https://github.com/justinhuangcode/browserx/releases/download/v#{version}/browserx-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      # sha256 will be filled by release CI
    end
  end

  def install
    bin.install "browserx"
  end

  test do
    assert_match "browserx", shell_output("#{bin}/browserx --version")
  end
end

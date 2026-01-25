# Homebrew formula for Rush shell
# To use this tap: brew tap opus-workshop/rush https://github.com/opus-workshop/rush
# Then: brew install rush

class Rush < Formula
  desc "High-performance, POSIX-compliant shell written in Rust"
  homepage "https://github.com/opus-workshop/rush"
  version "0.1.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/opus-workshop/rush/releases/download/v#{version}/rush-macos-aarch64.tar.gz"
      # sha256 "PLACEHOLDER_ARM64_SHA256"
    else
      url "https://github.com/opus-workshop/rush/releases/download/v#{version}/rush-macos-x86_64.tar.gz"
      # sha256 "PLACEHOLDER_X86_64_SHA256"
    end
  end

  on_linux do
    url "https://github.com/opus-workshop/rush/releases/download/v#{version}/rush-linux-x86_64.tar.gz"
    # sha256 "PLACEHOLDER_LINUX_SHA256"
  end

  def install
    bin.install "rush"
  end

  def caveats
    <<~EOS
      Rush has been installed!

      To use Rush as your default shell, add it to /etc/shells:
        echo "#{HOMEBREW_PREFIX}/bin/rush" | sudo tee -a /etc/shells

      Then change your shell:
        chsh -s #{HOMEBREW_PREFIX}/bin/rush

      For daemon mode (faster startup):
        rushd start    # Start the daemon
        rushd stop     # Stop the daemon
    EOS
  end

  test do
    assert_match "Hello from Rush", shell_output("#{bin}/rush -c 'echo Hello from Rush'")
    assert_match "/", shell_output("#{bin}/rush -c 'pwd'")
  end
end

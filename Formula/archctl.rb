class Archctl < Formula
  desc "CLI sidecar for arch-stack: extract + render architecture diagrams, manage skills"
  homepage "https://github.com/Rubentxu/arch-stack"
  url "https://github.com/Rubentxu/arch-stack/releases/download/v1.37.0/archctl-x86_64-unknown-linux-gnu.tar.gz"
  # NOTE: macOS binary URL will be added when release artifacts include darwin targets.
  # For now, users on macOS can use the Linux binary under Rosetta 2 or build from source.

  version "1.37.0"

  # Binary checksum (filled by CI when releasing; placeholder for now).
  # sha256 "TODO_CI_FILLED"

  def install
    bin.install "archctl" => "archctl"
  end

  test do
    system "#{bin}/archctl", "--version"
  end
end

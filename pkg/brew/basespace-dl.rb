class BasespaceDl < Formula
    version = '0.1.0'
    desc = 'Multi-account basespace file downloader'

    if OS.mac?
        url ""
        sha256 ""
    elsif OS.linux?
        url ""
        sha256 ""
    end

    def install
        bin.install "basespace-dl"
    end
end
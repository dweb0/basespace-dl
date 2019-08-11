class BasespaceDl < Formula
    version = '0.2.1'
    desc = 'Multi-account basespace file downloader'

    if OS.mac?
        url "https://github.com/dweb0/basespace-dl/releases/download/#{version}/basespace-dl-#{version}-x86_64-apple-darwin.zip"
        sha256 "69d8f2cffbc5759f6aedc66687e5c362bf6cfab408d07fb20458320edfc82c35"
    elsif OS.linux?
        url "https://github.com/dweb0/basespace-dl/releases/download/#{version}/basespace-dl-#{version}-x86_64-unknown-linux-gnu.zip"
        sha256 "3c413c6e6e51daf06e98a95ce050582cc1b2adcea96dd42801f45cba9faac559"
    end

    def install
        bin.install "basespace-dl"
    end
end
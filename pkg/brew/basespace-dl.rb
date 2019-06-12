class BasespaceDl < Formula
    version = '0.1.1'
    desc = 'Multi-account basespace file downloader'

    if OS.mac?
        url "https://github.com/dweb0/basespace-dl/releases/download/#{version}/basespace-dl-#{version}-x86_64-apple-darwin.zip"
        sha256 "2a4c926263001d63281821d7a1c34091df901ac70febadcf9f35f15a478ae75a"
    elsif OS.linux?
        url "https://github.com/dweb0/basespace-dl/releases/download/#{version}/basespace-dl-#{version}-x86_64-linux.zip"
        sha256 "d597b7e71404a57d1f9de4c15c4dd798e3f3de2dfb429068362be1274bcfaff9"
    end

    def install
        bin.install "basespace-dl"
    end
end
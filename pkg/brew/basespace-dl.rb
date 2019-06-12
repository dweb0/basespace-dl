class BasespaceDl < Formula
    version = '0.1.0'
    desc = 'Multi-account basespace file downloader'

    if OS.mac?
        url "https://github.com/dweb0/basespace-dl/releases/download/#{version}/basespace-dl-#{version}-x86_64-apple-darwin.zip"
        sha256 "6ac17907f83ec9243c48533c1d547ae993a64aca625ed106d45fc8d615e55370"
    elsif OS.linux?
        url "https://github.com/dweb0/basespace-dl/releases/download/#{version}/basespace-dl-#{version}-x86_64-linux.zip"
        sha256 "bc32278133b7dc30192ac4171a369767e49ad50a0987e12cc9c0915fbd5194ec"
    end

    def install
        bin.install "basespace-dl"
    end
end
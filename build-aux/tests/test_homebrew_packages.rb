# frozen_string_literal: true

# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
# Run with `brew ruby` after rendering the definitions into the trusted tap.
require "digest"
require "json"
require "formulary"
require "cask/cask_loader"
require "simulate_system"

version, assets, tap = ARGV
abort "Usage: brew ruby test_homebrew_packages.rb VERSION ASSETS [TAP]" if version.nil? || assets.nil?
tap ||= "dan-hart/tap"
assets = Pathname(assets)
base_url = "https://github.com/dan-hart/momentum/releases/download/v#{version}/"
checks = []

[:macos, :linux].product([:arm, :intel]).each do |os, arch|
  Homebrew::SimulateSystem.with(os: os, arch: arch) do
    Formulary.clear_cache
    formula = Formulary.factory("#{tap}/momentum-cli")
    suffix = if os == :macos
      "macos-universal"
    else
      "linux-#{(arch == :arm) ? "aarch64" : "x86_64"}"
    end
    archive = "mo-v#{version}-#{suffix}.tar.gz"
    raise "Wrong CLI version for #{os}/#{arch}" if formula.version.to_s != version
    raise "Wrong CLI URL for #{os}/#{arch}" if formula.stable.url != base_url + archive
    raise "Wrong CLI digest for #{os}/#{arch}" if formula.stable.checksum.to_s != Digest::SHA256.file(assets/archive).hexdigest

    checks << { os: os, arch: arch, archive: archive, version: formula.version.to_s }
  end
end

[:arm, :intel].each do |arch|
  Homebrew::SimulateSystem.with(os: :macos, arch: arch) do
    cask = Cask::CaskLoader.load("#{tap}/momentum")
    archive = "Momentum-v#{version}-macos.zip"
    raise "Wrong app version for #{arch}" if cask.version.to_s != version
    raise "Wrong app URL for #{arch}" if cask.url.to_s != base_url + archive
    raise "Wrong app digest for #{arch}" if cask.sha256.to_s != Digest::SHA256.file(assets/archive).hexdigest

    checks << { os: :macos, arch: arch, archive: archive, version: cask.version.to_s }
  end
end

puts JSON.pretty_generate(checks)

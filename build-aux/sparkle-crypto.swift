// SPDX-License-Identifier: GPL-3.0-or-later
// CryptoKit verifies independently of Sparkle's signing tool. Secrets arrive only on stdin.
import CryptoKit
import Foundation

struct Input: Decodable {
    var publicKey: String
    var privateKey: String?
    var archive: String?
    var signature: String?
}
do {
    let input = try JSONDecoder().decode(Input.self, from: FileHandle.standardInput.readDataToEndOfFile())
    guard let publicData = Data(base64Encoded: input.publicKey), publicData.count == 32 else { exit(1) }
    let key = try Curve25519.Signing.PublicKey(rawRepresentation: publicData)
    if let seed = input.privateKey {
        guard let raw = Data(base64Encoded: seed), raw.count == 32 else { exit(1) }
        let privateKey = try Curve25519.Signing.PrivateKey(rawRepresentation: raw)
        guard privateKey.publicKey.rawRepresentation == publicData else { exit(1) }
    } else {
        guard let path = input.archive, let encoded = input.signature,
              let signature = Data(base64Encoded: encoded), signature.count == 64 else { exit(1) }
        let archive = try Data(contentsOf: URL(fileURLWithPath: path), options: .mappedIfSafe)
        guard key.isValidSignature(signature, for: archive) else { exit(1) }
    }
} catch { exit(1) }

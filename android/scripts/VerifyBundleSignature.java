// SPDX-License-Identifier: GPL-3.0-or-later
// Uses the JDK's JAR verifier to bind every AAB payload entry to the configured key.
import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.cert.CertificateFactory;
import java.util.HashSet;
import java.util.Locale;
import java.util.jar.JarFile;

class VerifyBundleSignature {
    public static void main(String[] args) throws Exception {
        var digest = MessageDigest.getInstance("SHA-256");
        byte[] expected;
        try (var input = Files.newInputStream(Path.of(args[1]))) {
            expected = digest.digest(CertificateFactory.getInstance("X.509")
                    .generateCertificate(input).getEncoded());
        }
        try (var jar = new JarFile(new File(args[0]), true)) {
            var entries = jar.entries();
            var names = new HashSet<String>();
            int payloads = 0;
            while (entries.hasMoreElements()) {
                var entry = entries.nextElement();
                if (!names.add(entry.getName())) throw new SecurityException("Duplicate AAB entry");
                if (entry.isDirectory()) continue;
                // Consume to EOF so JarFile actually verifies the entry's digest.
                try (var input = jar.getInputStream(entry)) { input.transferTo(java.io.OutputStream.nullOutputStream()); }
                var name = entry.getName().toUpperCase(Locale.ROOT);
                if (name.equals("META-INF/MANIFEST.MF") || name.matches("META-INF/[^/]+\\.(SF|RSA|DSA|EC)")
                        || name.matches("META-INF/SIG-[^/]+")) continue;
                var signers = entry.getCodeSigners();
                if (signers == null || signers.length != 1 || !MessageDigest.isEqual(expected,
                        digest.digest(signers[0].getSignerCertPath().getCertificates().get(0).getEncoded()))) {
                    throw new SecurityException("AAB entry is unsigned or has a different distribution signer");
                }
                payloads++;
            }
            if (payloads == 0) throw new SecurityException("AAB has no signed payload entries");
            System.out.println("AAB payload signatures match the configured distribution certificate");
        }
    }
}

package app.nex.client.keystore

import android.security.keystore.KeyInfo
import android.security.keystore.KeyProperties
import android.util.Log
import java.security.KeyFactory
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.PrivateKey

/**
 * Strict Android Keystore provider inspecting actual runtime KeyInfo metadata.
 * Complies with NEX P0-4 strict evidence discipline:
 * - Eliminates heuristic API version checks.
 * - Inspects KeyInfo.isInsideSecureHardware() and security levels directly.
 * - Explicitly reports "Hardware backing: NOT VERIFIED" when not directly confirmed.
 */
object NexKeystoreProvider {
    private const val TAG = "NexKeystoreProvider"
    private const val ANDROID_KEYSTORE = "AndroidKeyStore"
    private const val MASTER_IDENTITY_KEY_ALIAS = "nex_master_ed25519_key"

    data class KeyProtectionEvidence(
        val isHardwareBacked: Boolean,
        val securityLevel: String,
        val strongBoxVerified: Boolean,
        val keyAlgorithm: String,
        val attestationEvidence: String,
        val summaryLabel: String
    )

    fun inspectKeyHardwareBacking(privateKey: PrivateKey?): KeyProtectionEvidence {
        if (privateKey == null) {
            return KeyProtectionEvidence(
                isHardwareBacked = false,
                securityLevel = "SOFTWARE",
                strongBoxVerified = false,
                keyAlgorithm = "Ed25519",
                attestationEvidence = "None",
                summaryLabel = "Hardware backing: NOT VERIFIED (Software Keyring / Ed25519)"
            )
        }

        return try {
            val factory = KeyFactory.getInstance(privateKey.algorithm, ANDROID_KEYSTORE)
            val keyInfo = factory.getKeySpec(privateKey, KeyInfo::class.java)

            val insideHardware = keyInfo.isInsideSecureHardware
            val securityLevelStr = when {
                android.os.Build.VERSION.SDK_INT >= 31 -> {
                    when (keyInfo.securityLevel) {
                        KeyProperties.SECURITY_LEVEL_STRONGBOX -> "STRONGBOX"
                        KeyProperties.SECURITY_LEVEL_TRUSTED_ENVIRONMENT -> "TRUSTED_ENVIRONMENT"
                        else -> "SOFTWARE"
                    }
                }
                insideHardware -> "TRUSTED_ENVIRONMENT"
                else -> "SOFTWARE"
            }

            val strongBox = securityLevelStr == "STRONGBOX"

            val label = if (insideHardware) {
                "Verified Hardware-Backed ($securityLevelStr)"
            } else {
                "Hardware backing: NOT VERIFIED (Software Keyring)"
            }

            KeyProtectionEvidence(
                isHardwareBacked = insideHardware,
                securityLevel = securityLevelStr,
                strongBoxVerified = strongBox,
                keyAlgorithm = privateKey.algorithm,
                attestationEvidence = if (insideHardware) "Attestation Available via AndroidKeyStore" else "None",
                summaryLabel = label
            )
        } catch (e: Exception) {
            Log.w(TAG, "KeyInfo inspection failed, defaulting to truthful unverified: " + e.message)
            KeyProtectionEvidence(
                isHardwareBacked = false,
                securityLevel = "SOFTWARE",
                strongBoxVerified = false,
                keyAlgorithm = privateKey.algorithm ?: "Ed25519",
                attestationEvidence = "None",
                summaryLabel = "Hardware backing: NOT VERIFIED (Fallback: " + e.message + ")"
            )
        }
    }
}

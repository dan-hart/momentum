// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum;

import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Process;
import android.widget.TextView;

/** Standalone test-APK recipient: no target-APK Kotlin runtime dependency in its separate UID. */
public final class ClipboardRecipientActivity extends Activity {
    private ClipboardManager clipboard;
    @Override public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        clipboard = getSystemService(ClipboardManager.class);
        TextView label = new TextView(this);
        label.setText("Disposable clipboard recipient");
        setContentView(label);
        new Handler(Looper.getMainLooper()).postDelayed(() -> new Thread(this::receive).start(), 250);
    }

    private boolean allowed(Uri uri, int mode) {
        return checkUriPermission(uri, Process.myPid(), Process.myUid(), mode)
                == PackageManager.PERMISSION_GRANTED;
    }

    private void receive() {
        boolean sharing = Intent.ACTION_SEND.equals(getIntent().getAction());
        String resultAction = getIntent().getStringExtra("resultAction");
        String targetPackage = getIntent().getStringExtra("targetPackage");
        Intent result = new Intent(resultAction == null ? "com.codedbydan.momentum.test.NOTES_SHARE_RESULT" : resultAction)
                .setPackage(targetPackage == null ? "com.codedbydan.momentum" : targetPackage)
                .putExtra("uid", Process.myUid());
        try {
            ClipData clip = sharing ? getIntent().getClipData() : null;
            Uri expectedUri = sharing ? clip.getItemAt(0).getUri() : Uri.parse(getIntent().getStringExtra("uri"));
            result.putExtra("uri", expectedUri.toString());
            result.putExtra("readBefore", allowed(expectedUri, Intent.FLAG_GRANT_READ_URI_PERMISSION));
            if (!sharing) clip = clipboard.getPrimaryClip();
            if (clip == null) throw new IllegalStateException();
            result.putExtra("readAfter", allowed(expectedUri, Intent.FLAG_GRANT_READ_URI_PERMISSION))
                    .putExtra("writeAfter", allowed(expectedUri, Intent.FLAG_GRANT_WRITE_URI_PERMISSION));
            String text = clip.getItemAt(0).coerceToText(this).toString();
            String fixture = getIntent().getStringExtra("fixture");
            if (fixture == null && sharing) fixture = "unicode";
            StringBuilder expected = new StringBuilder();
            int count;
            String piece;
            if ("ascii".equals(fixture)) { count = 2 * 1024 * 1024; piece = "A"; }
            else if ("unicode".equals(fixture)) { count = 270 * 1024; piece = "\uD83D\uDCDD"; }
            else throw new IllegalArgumentException();
            for (int index = 0; index < count; index++) expected.append(piece);
            result.putExtra("exact", text.contentEquals(expected)).putExtra("length", text.length());
        } catch (Exception error) { result.putExtra("failure", error.getClass().getName()); }
        sendBroadcast(result);
        runOnUiThread(this::finish);
    }
}

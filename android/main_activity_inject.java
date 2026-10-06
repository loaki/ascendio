// Spliced into miniquad's MainActivity by cargo-quad-apk (see quad.toml):
// each "//%" block lands at the matching marker. src/platform/ calls these.

//% IMPORTS
import android.app.NotificationManager;
import android.content.pm.PackageManager;
import android.provider.Settings;
import android.app.AlertDialog;
import android.content.DialogInterface;
import android.text.InputFilter;
import android.widget.EditText;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.concurrent.ConcurrentHashMap;
//% END

//% MAIN_ACTIVITY_BODY
    public void scheduleGenome(long delayMs, String body) {
        GenomeJob.schedule(this, delayMs, body);
    }

    public void cancelGenome() {
        GenomeJob.cancel(this);
    }

    public boolean notificationsAllowed() {
        if (Build.VERSION.SDK_INT >= 33
            && checkSelfPermission("android.permission.POST_NOTIFICATIONS")
                != PackageManager.PERMISSION_GRANTED) {
            return false;
        }
        NotificationManager nm = (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
        return nm.areNotificationsEnabled();
    }

    public void askNotifications() {
        if (Build.VERSION.SDK_INT < 33 || notificationsAllowed()) {
            return;
        }
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                requestPermissions(new String[] { "android.permission.POST_NOTIFICATIONS" }, 7);
            }
        });
    }

    public void openNotificationSettings() {
        Intent i;
        if (Build.VERSION.SDK_INT >= 26) {
            i = new Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS);
            i.putExtra(Settings.EXTRA_APP_PACKAGE, getPackageName());
        } else {
            i = new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS);
            i.setData(android.net.Uri.fromParts("package", getPackageName(), null));
        }
        startActivity(i);
    }

    // The "new version" banner (platform/update.rs) opens the release page.
    public void openUrl(String url) {
        Intent i = new Intent(Intent.ACTION_VIEW, android.net.Uri.parse(url));
        i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        startActivity(i);
    }

    // --- the leaderboard's HTTP (platform/net.rs) ------------------------------
    // A request runs on its own thread; the game polls httpStatus each frame
    // (0 in flight, 1 if nothing answered, else the HTTP status), then takes
    // the body once.

    private final ConcurrentHashMap<Integer, String[]> httpDone = new ConcurrentHashMap<Integer, String[]>();
    private int httpNext = 1;

    public int httpStart(final String method, final String url, final String body) {
        final int id = httpNext++;
        new Thread(new Runnable() {
            @Override
            public void run() {
                String[] result;
                try {
                    HttpURLConnection c = (HttpURLConnection) new URL(url).openConnection();
                    c.setRequestMethod(method);
                    c.setConnectTimeout(10000);
                    c.setReadTimeout(10000);
                    if (body.length() > 0) {
                        c.setDoOutput(true);
                        c.setRequestProperty("Content-Type", "application/json");
                        OutputStream out = c.getOutputStream();
                        out.write(body.getBytes("UTF-8"));
                        out.close();
                    }
                    int code = c.getResponseCode();
                    InputStream in = code < 400 ? c.getInputStream() : c.getErrorStream();
                    String text = "";
                    if (in != null) {
                        ByteArrayOutputStream buf = new ByteArrayOutputStream();
                        byte[] chunk = new byte[4096];
                        int n;
                        while ((n = in.read(chunk)) > 0) {
                            buf.write(chunk, 0, n);
                        }
                        in.close();
                        text = buf.toString("UTF-8");
                    }
                    c.disconnect();
                    result = new String[] { Integer.toString(code), text };
                } catch (Exception e) {
                    result = new String[] { "1", "" };
                }
                httpDone.put(id, result);
            }
        }).start();
        return id;
    }

    public int httpStatus(int id) {
        String[] r = httpDone.get(id);
        return r == null ? 0 : Integer.parseInt(r[0]);
    }

    public String httpTake(int id) {
        String[] r = httpDone.remove(id);
        return r == null ? "" : r[1];
    }

    // --- the name dialog (platform/net.rs) -------------------------------------
    // promptStatus: 0 nothing asked, 1 open, 2 answered (text, or null if
    // cancelled); promptTake hands the answer over and goes back to 0.

    private volatile int promptState = 0;
    private volatile String promptAnswer = null;

    public int promptStatus() {
        return promptState;
    }

    public String promptTake() {
        String a = promptAnswer;
        promptAnswer = null;
        promptState = 0;
        return a;
    }

    public void promptOpen(final String title, final String current) {
        promptState = 1;
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                final EditText input = new EditText(MainActivity.this);
                input.setText(current);
                input.setSingleLine(true);
                input.setSelectAllOnFocus(true);
                input.setFilters(new InputFilter[] { new InputFilter.LengthFilter(16) });
                AlertDialog dialog = new AlertDialog.Builder(MainActivity.this)
                    .setTitle(title)
                    .setView(input)
                    .setPositiveButton("OK", new DialogInterface.OnClickListener() {
                        @Override
                        public void onClick(DialogInterface d, int which) {
                            promptAnswer = input.getText().toString();
                            promptState = 2;
                        }
                    })
                    .setNegativeButton("Cancel", new DialogInterface.OnClickListener() {
                        @Override
                        public void onClick(DialogInterface d, int which) {
                            promptAnswer = null;
                            promptState = 2;
                        }
                    })
                    .setOnCancelListener(new DialogInterface.OnCancelListener() {
                        @Override
                        public void onCancel(DialogInterface d) {
                            promptAnswer = null;
                            promptState = 2;
                        }
                    })
                    .create();
                dialog.show();
            }
        });
    }

    // --- the trusted clock (platform/clock.rs) ---------------------------------
    // Uptime that changing the phone's time doesn't move, and the restart
    // counter that says when it started over (-1 if unknown).

    public long uptimeMs() {
        return android.os.SystemClock.elapsedRealtime();
    }

    public int bootCount() {
        try {
            return Settings.Global.getInt(getContentResolver(), Settings.Global.BOOT_COUNT);
        } catch (Exception e) {
            return -1;
        }
    }
//% END

// Spliced into miniquad's MainActivity by cargo-quad-apk (see quad.toml):
// each "//%" block lands at the matching marker. src/notify.rs calls these.

//% IMPORTS
import android.app.NotificationManager;
import android.content.pm.PackageManager;
import android.provider.Settings;
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
//% END

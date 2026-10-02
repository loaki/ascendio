package TARGET_PACKAGE_NAME;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.job.JobInfo;
import android.app.job.JobParameters;
import android.app.job.JobScheduler;
import android.app.job.JobService;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.os.PersistableBundle;

// Posts "a new genome is ready" when a wait ends, with the game closed.
// `src/notify.rs` schedules it through MainActivity (main_activity_inject.java);
// Cargo.toml declares it as a service. `setPersisted` keeps the job across a
// reboot, which is why the app asks for RECEIVE_BOOT_COMPLETED.
public class GenomeJob extends JobService {
    private static final int JOB_ID = 1;
    private static final int NOTIFICATION_ID = 1;
    private static final String CHANNEL = "genome";

    @Override
    public boolean onStartJob(JobParameters params) {
        post(this, params.getExtras().getString("body"));
        return false;
    }

    @Override
    public boolean onStopJob(JobParameters params) {
        return false;
    }

    public static void schedule(Context ctx, long delayMs, String body) {
        PersistableBundle extras = new PersistableBundle();
        extras.putString("body", body);
        long delay = Math.max(0, delayMs);
        JobInfo job = new JobInfo.Builder(JOB_ID, new ComponentName(ctx, GenomeJob.class))
            .setMinimumLatency(delay)
            // Without a deadline an idle phone may sit on it for a long time.
            .setOverrideDeadline(delay + 60000)
            .setPersisted(true)
            .setExtras(extras)
            .build();
        scheduler(ctx).schedule(job);
    }

    public static void cancel(Context ctx) {
        scheduler(ctx).cancel(JOB_ID);
        manager(ctx).cancel(NOTIFICATION_ID);
    }

    @SuppressWarnings("deprecation")
    static void post(Context ctx, String body) {
        NotificationManager nm = manager(ctx);
        Notification.Builder b;
        if (Build.VERSION.SDK_INT >= 26) {
            nm.createNotificationChannel(new NotificationChannel(
                CHANNEL, "Genome ready", NotificationManager.IMPORTANCE_DEFAULT));
            b = new Notification.Builder(ctx, CHANNEL);
        } else {
            b = new Notification.Builder(ctx);
        }
        // Tapping it brings the game back, or starts it.
        Intent open = ctx.getPackageManager().getLaunchIntentForPackage(ctx.getPackageName());
        PendingIntent tap = PendingIntent.getActivity(ctx, 0, open,
            PendingIntent.FLAG_IMMUTABLE | PendingIntent.FLAG_UPDATE_CURRENT);
        // Looked up by name: the R class belongs to the build, not to us.
        int icon = ctx.getResources().getIdentifier("ic_stat_genome", "drawable", ctx.getPackageName());
        if (icon == 0) {
            icon = ctx.getApplicationInfo().icon;
        }
        b.setSmallIcon(icon)
            .setColor(0xFF5BC8F5)
            .setContentTitle("A new genome is ready to evolve")
            .setContentText(body)
            .setContentIntent(tap)
            .setAutoCancel(true);
        nm.notify(NOTIFICATION_ID, b.build());
    }

    private static JobScheduler scheduler(Context ctx) {
        return (JobScheduler) ctx.getSystemService(Context.JOB_SCHEDULER_SERVICE);
    }

    private static NotificationManager manager(Context ctx) {
        return (NotificationManager) ctx.getSystemService(Context.NOTIFICATION_SERVICE);
    }
}

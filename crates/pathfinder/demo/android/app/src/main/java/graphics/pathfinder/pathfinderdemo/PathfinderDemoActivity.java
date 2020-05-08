package graphics.pathfinder.pathfinderdemo;

import android.Manifest;
import android.annotation.SuppressLint;
import android.app.Activity;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.hardware.Sensor;
import android.hardware.SensorEvent;
import android.hardware.SensorEventListener;
import android.hardware.SensorManager;
import android.os.Build;
import android.support.annotation.NonNull;
import android.support.annotation.RequiresApi;
import android.support.v4.app.ActivityCompat;
import android.support.v4.content.ContextCompat;
import android.os.Bundle;
import android.util.Log;
import android.view.GestureDetector;
import android.view.MotionEvent;
import android.view.ScaleGestureDetector;
import android.view.View;

/**
 * An example full-screen activity that shows and hides the system UI (i.e.
 * status bar and navigation/system bar) with user interaction.
 */
public class PathfinderDemoActivity extends Activity {
    private PathfinderDemoRenderer mRenderer;

    /**
     * Some older devices needs a small delay between UI widget updates
     * and a change of the status and navigation bar.
     */
    private PathfinderDemoSurfaceView mContentView;

    private GestureDetector mGestureDetector;
    private ScaleGestureDetector mScaleGestureDetector;

    ComponentName mVRListenerComponentName;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        if (ContextCompat.checkSelfPermission(this,
                Manifest.permission.READ_EXTERNAL_STORAGE) != PackageManager.PERMISSION_GRANTED) {
            String[] perms = new String[1];
            perms[0] = Manifest.permission.READ_EXTERNAL_STORAGE;
            ActivityCompat.requestPermissions(this, perms,
                    1);
        } else {
            init();
        }
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, @NonNull String[] permissions, @NonNull int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);

        if (permissions[0] == Manifest.permission.READ_EXTERNAL_STORAGE)
            init();
    }

    @RequiresApi(api = Build.VERSION_CODES.N)
    void setVRMode(boolean enabled) {
        mContentView.setStereoModeEnabled(enabled);
        mContentView.setDistortionCorrectionEnabled(false);
    }

    @RequiresApi(api = Build.VERSION_CODES.N)
    @SuppressLint("ClickableViewAccessibility")
    private void init() {
        mVRListenerComponentName = new ComponentName("graphics.pathfinder.pathfinderdemo",
                "graphics.pathfinder.pathfinderdemo.PathfinderDemoVRListenerService");

        setContentView(R.layout.activity_pathfinder);

        mContentView = findViewById(R.id.fullscreen_content);
        setVRMode(false);

        mContentView.setEGLContextClientVersion(3);
        mRenderer = new PathfinderDemoRenderer(this);
        mContentView.setRenderer(mRenderer);

        GestureDetector.SimpleOnGestureListener gestureListener =
                new GestureDetector.SimpleOnGestureListener() {
            public boolean onScroll(final MotionEvent from,
                                    final MotionEvent to,
                                    final float deltaX,
                                    final float deltaY) {
                final int x = Math.round(to.getX());
                final int y = Math.round(to.getY());
                PathfinderDemoRenderer.pushMouseDraggedEvent(x, y);
                return true;
            }

            public boolean onDown(final MotionEvent event) {
                final int x = Math.round(event.getX());
                final int y = Math.round(event.getY());
                PathfinderDemoRenderer.pushMouseDownEvent(x, y);
                return true;
            }
        };
        mGestureDetector = new GestureDetector(getApplicationContext(), gestureListener);

        ScaleGestureDetector.SimpleOnScaleGestureListener scaleGestureListener =
                new ScaleGestureDetector.SimpleOnScaleGestureListener() {
            public boolean onScale(final ScaleGestureDetector detector) {
                int focusX = Math.round(detector.getFocusX());
                int focusY = Math.round(detector.getFocusY());
                float factor = (detector.getScaleFactor() - 1.0f) * 0.5f;
                PathfinderDemoRenderer.pushZoomEvent(factor, focusX, focusY);
                return true;
            }
        };
        mScaleGestureDetector = new ScaleGestureDetector(getApplicationContext(),
                scaleGestureListener);

        mContentView.setOnTouchListener(new View.OnTouchListener() {
            @Override
            public boolean onTouch(final View view, final MotionEvent event) {
                boolean result = mScaleGestureDetector.onTouchEvent(event);
                if (!mScaleGestureDetector.isInProgress())
                    result = mGestureDetector.onTouchEvent(event) || result;
                return result;
            }
        });

        final SensorManager sensorManager = (SensorManager)
                getSystemService(Context.SENSOR_SERVICE);
        final Sensor rotationSensor = sensorManager.getDefaultSensor(Sensor.TYPE_ROTATION_VECTOR);
        sensorManager.registerListener(new SensorEventListener() {
            private boolean mInitialized;
            private float mPitch;
            private float mYaw;

            @Override
            public void onSensorChanged(SensorEvent event) {
                // https://en.wikipedia.org/wiki/Conversion_between_quaternions_and_Euler_angles#Quaternion_to_Euler_Angles_Conversion
                final float[] q = event.values;
                final float pitch = (float)Math.asin(2.0 * (q[0] * q[2] - q[3] * q[1]));
                final float yaw = (float)Math.atan2(2.0 * (q[0] * q[3] + q[1] * q[2]),
                                                    1.0 - 2.0 * (q[2] * q[2] + q[3] * q[3]));

                final float deltaPitch = pitch - mPitch;
                final float deltaYaw = yaw - mYaw;

                mPitch = pitch;
                mYaw = yaw;

                if (!mInitialized) {
                    mInitialized = true;
                    return;
                }

                PathfinderDemoRenderer.pushLookEvent(-deltaPitch, deltaYaw);
            }

            @Override
            public void onAccuracyChanged(Sensor sensor, int accuracy) {

            }
        }, rotationSensor, 5000);
    }

    @Override
    protected void onPostCreate(Bundle savedInstanceState) {
        super.onPostCreate(savedInstanceState);
    }

    public void presentOpenSVGDialog() {
        final Intent intent = new Intent(this, PathfinderDemoFileBrowserActivity.class);
        startActivity(intent);
    }
}

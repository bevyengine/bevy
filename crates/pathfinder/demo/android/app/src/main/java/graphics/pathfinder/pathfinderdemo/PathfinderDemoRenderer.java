package graphics.pathfinder.pathfinderdemo;

import android.content.Intent;
import android.content.pm.PackageManager;
import android.util.Log;

import com.google.vr.sdk.base.Eye;
import com.google.vr.sdk.base.GvrView;
import com.google.vr.sdk.base.HeadTransform;
import com.google.vr.sdk.base.Viewport;
import javax.microedition.khronos.egl.EGLConfig;

public class PathfinderDemoRenderer extends Object implements GvrView.Renderer {
    private final PathfinderDemoActivity mActivity;
    private boolean mInitialized;
    private boolean mInVRMode;

    private static native void init(PathfinderDemoActivity activity,
                                    PathfinderDemoResourceLoader resourceLoader,
                                    int width,
                                    int height);

    private static native int prepareFrame();

    private static native void drawScene();

    private static native void finishDrawingFrame();

    public static native void pushWindowResizedEvent(int width, int height);

    public static native void pushMouseDownEvent(int x, int y);

    public static native void pushMouseDraggedEvent(int x, int y);

    public static native void pushZoomEvent(float scale, int centerX, int centerY);

    public static native void pushLookEvent(float pitch, float yaw);

    public static native void pushOpenSVGEvent(String path);

    static {
        System.loadLibrary("pathfinder_android_demo");
    }

    PathfinderDemoRenderer(PathfinderDemoActivity activity) {
        super();
        mActivity = activity;
        mInitialized = false;
    }

    @Override
    public void onDrawFrame(HeadTransform headTransform, Eye leftEye, Eye rightEye) {
        final boolean inVR = prepareFrame() > 1;
        if (inVR != mInVRMode) {
            mInVRMode = inVR;
            try {
                mActivity.setVrModeEnabled(mInVRMode, mActivity.mVRListenerComponentName);
                mActivity.setVRMode(inVR);
            } catch (PackageManager.NameNotFoundException exception) {
                throw new RuntimeException(exception);
            }
        }

        drawScene();
        finishDrawingFrame();
    }

    @Override
    public void onFinishFrame(Viewport viewport) {

    }

    @Override
    public void onSurfaceChanged(int width, int height) {
        if (!mInitialized) {
            init(mActivity,
                    new PathfinderDemoResourceLoader(mActivity.getAssets()),
                    width,
                    height);
            mInitialized = true;
        } else {
            pushWindowResizedEvent(width, height);
        }

    }

    @Override
    public void onSurfaceCreated(EGLConfig config) {

    }

    @Override
    public void onRendererShutdown() {

    }
}

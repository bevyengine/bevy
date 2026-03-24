package org.bevyengine.example

import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.google.androidgamesdk.GameActivity

/**
 * Load rust library and handle android specifics to integrate with it.
 *
 *
 * The library is loaded at class initialization and provided by jniLibs.
 */
class MainActivity : GameActivity() {
    /**
     * Hide system UI if the app window is focused.
     *
     *
     * Called when the current Window of the activity gains or loses focus.
     */
    override fun onWindowFocusChanged(hasFocus: Boolean) {
        // Call parent class implementation of onWindowFocusChanged to make sure that we are updating correctly.
        super.onWindowFocusChanged(hasFocus)

        if (hasFocus) {
            hideSystemUi()
        }
    }

    /**
     * Hide system UI.
     */
    private fun hideSystemUi() {
        val windowInsetsController =
            WindowCompat.getInsetsController(window, window.decorView)

        // Show bars if swiping
        windowInsetsController.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        // Hide both the status bar and the navigation bar.
        windowInsetsController.hide(WindowInsetsCompat.Type.systemBars())
    }

    companion object {
        // Load rust library
        init {
            System.loadLibrary("bevy_mobile_example")
        }
    }
}

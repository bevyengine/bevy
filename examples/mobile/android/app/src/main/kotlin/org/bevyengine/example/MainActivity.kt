package org.bevyengine.example

import android.os.Bundle
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
     * Enable edge to edge when the activity is starting.
     *
     *
     * This is the default behavior after Android SDK 34 but is needed for backwards compatibility.
     */
    override fun onCreate(savedInstanceState: Bundle?) {
        // Call parent class implementation of onCreate.
        super.onCreate(savedInstanceState)

        WindowCompat.enableEdgeToEdge(window)
    }

    /**
     * Hide system UI if the current window gains or loses focus.
     */
    override fun onWindowFocusChanged(hasFocus: Boolean) {
        // Call parent class implementation of onWindowFocusChanged.
        super.onWindowFocusChanged(hasFocus)

        if (hasFocus) {
            hideSystemUi()
        }
    }

    /**
     * Hide system UI.
     */
    private fun hideSystemUi() {
        window.decorView.post {
            val controller = WindowCompat.getInsetsController(window, window.decorView)
            // Show bars if swiping
            controller.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            // Hide both the status bar and the navigation bar.
            controller.hide(WindowInsetsCompat.Type.systemBars())
        }
    }

    companion object {
        // Load rust library
        init {
            System.loadLibrary("bevy_mobile_example")
        }
    }
}

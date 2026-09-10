# Bevy's Mobile Example

## Android

- **Build Library:**

  Run the following command to build the native library for Android:

  ```bash
  cargo ndk -t arm64-v8a -t armeabi-v7a -t x86 -t x86_64 -o android/app/src/main/jniLibs build --package bevy_mobile_example
  ```

- **Run Using Android Studio:**

  1. Open **Android Studio** and navigate to `android`.
  2. Start an Android emulator or connect a physical device.
  3. Run or debug the application on the selected device.

- **Run Using Shell Commands:**

  1. Ensure you have a device available for testing:
     - **Emulator:** Create and launch an Android Virtual Device (AVD).
     - **USB Device:** Connect an Android device via USB.
     - **Wireless Device:** Pair your device using **Android Studio** (recommended for simplicity).
  
  2. Start the ADB server and verify the connection:
  
     ```bash
     adb start-server
     adb devices
     ```
  
  3. Navigate to the Android project directory:
  
     ```bash
     cd android
     ```
  
  4. Ensure `./gradlew` has execution permissions:
  
     ```bash
     chmod +x ./gradlew
     ```
  
  5. Build the application:
  
     - **Debug:**
  
       ```bash
       ./gradlew assembleDebug
       ```
  
     - **Release:**
  
       ```bash
       ./gradlew assembleRelease
       ```
  
     - **Bundle (requires signing configuration in Gradle):**
  
       ```bash
       ./gradlew bundleRelease
       ```
  
  6. Install the application on the device:
  
     - **Debug:**
  
       ```bash
       adb install -r app/build/outputs/apk/debug/app-debug.apk
       ```
  
     - **Release:**
  
       ```bash
       adb install -r app/build/outputs/apk/release/app-release-unsigned.apk
       ```
  
       **Note:** The release build requires signing before installation.
  
  7. Launch the application:
  
     ```bash
     adb shell am start -n org.bevyengine.mobile/.MainActivity
     ```

## iOS

- **Run Using Xcode:**

  1. Open **Xcode** by opening `bevh_mobile_example.xcodeproj`.
  2. Select the target device (Simulator or Physical Device).
  3. Click **Run** ▶️ or **Debug** 🛠️ to launch the application.

- **Run Using Shell Commands:**

  - **Run on the First Available Simulator:**
  
    Simply run:
  
    ```sh
    make
    ```
  
    This executes the default `run` command in the `Makefile`, launching the app on the first available simulator.
  
  - **Run on a Specific Device:**
  
    1. **Find the Device ID:**
  
       - For **simulators**, run:
  
         ```sh
         xcrun simctl list devices
         ```
  
       - For **physical devices**, run:
  
         ```sh
         xcrun xctrace list devices
         ```
  
       - Copy the desired **Device ID**, e.g., `912BFD4B-9AFB-4DDE-983A-1816245DB2DA`.
  
    2. **Run the App on the Selected Device:**
  
       ```sh
       make run DEVICE_ID=912BFD4B-9AFB-4DDE-983A-1816245DB2DA
       ```

## Desktop

This section shows how to develop both mobile and desktop apps within the same Cargo.toml project. Developing on desktop is faster and easier, and you can simulate mobile resolution. When you're ready to release for mobile, you can simply build it for mobile at that point.

- **To run from the Bevy root:**

  ```bash
  cargo run -p bevy_mobile_example
  ```

- **To run from the Bevy examples directory for mobile:**

  ```bash
  cargo run
  ```

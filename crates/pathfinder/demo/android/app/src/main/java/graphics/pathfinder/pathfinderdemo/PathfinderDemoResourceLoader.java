package graphics.pathfinder.pathfinderdemo;

import android.content.res.AssetManager;
import android.util.Log;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;

public class PathfinderDemoResourceLoader {
    private AssetManager m_assetManager;

    PathfinderDemoResourceLoader(AssetManager assetManager) {
        m_assetManager = assetManager;
    }

    ByteBuffer slurp(String path) {
        try {
            InputStream inputStream = m_assetManager.open("resources/" + path);
            ByteArrayOutputStream outputStream = new ByteArrayOutputStream();

            byte[] buffer = new byte[4096];
            while (true) {
                int nRead = inputStream.read(buffer, 0, buffer.length);
                if (nRead == -1)
                    break;
                outputStream.write(buffer, 0, nRead);
            }

            byte[] outputBytes = outputStream.toByteArray();
            ByteBuffer resultBuffer = ByteBuffer.allocateDirect(outputStream.size());
            resultBuffer.put(outputBytes);
            return resultBuffer;
        } catch (IOException exception) {
            Log.e("Pathfinder", "Resource not found: " + path);
            return null;
        }
    }
}

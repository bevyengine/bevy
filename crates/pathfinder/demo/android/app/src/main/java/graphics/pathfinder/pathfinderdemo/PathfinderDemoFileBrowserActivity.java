package graphics.pathfinder.pathfinderdemo;

import android.content.res.AssetManager;
import android.os.Bundle;
import android.app.Activity;
import android.view.View;
import android.widget.AdapterView;
import android.widget.ArrayAdapter;
import android.widget.ListView;
import android.widget.TextView;

import java.io.IOException;

public class PathfinderDemoFileBrowserActivity extends Activity {
    private ListView mBrowserView;

    private static String SVG_RESOURCE_PATH = "svg/";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_pathfinder_demo_file_browser);

        mBrowserView = findViewById(R.id.fileBrowserBrowser);

        try {
            final AssetManager assetManager = getAssets();
            final String[] svgFilenames = assetManager.list("resources/" + SVG_RESOURCE_PATH);
            final ArrayAdapter<String> adapter = new ArrayAdapter<String>(
                    this,
                    R.layout.layout_pathfinder_demo_file_browser_list_item,
                    svgFilenames);
            mBrowserView.setAdapter(adapter);
        } catch (IOException exception) {
            throw new RuntimeException(exception);
        }

        mBrowserView.setOnItemClickListener(new AdapterView.OnItemClickListener() {
            @Override
            public void onItemClick(AdapterView<?> parent, View view, int position, long id) {
                TextView textView = (TextView)view;
                PathfinderDemoRenderer.pushOpenSVGEvent(SVG_RESOURCE_PATH + textView.getText());
                finish();
            }
        });
    }
}

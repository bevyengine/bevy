#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!
#version 450
































#extension GL_GOOGLE_include_directive : enable

precision highp float;
precision highp sampler2D;


















































uniform sampler2D uColorTexture0;
uniform sampler2D uMaskTexture0;
uniform sampler2D uDestTexture;
uniform sampler2D uGammaLUT;


uniform vec4 uFilterParams0;
uniform vec4 uFilterParams1;
uniform vec4 uFilterParams2;
uniform vec2 uFramebufferSize;
uniform vec2 uColorTexture0Size;
uniform int uCtrl;

in vec3 vMaskTexCoord0;
in vec2 vColorTexCoord0;
in vec4 vBaseColor;
in float vTileCtrl;

out vec4 oFragColor;








vec4 sampleColor(sampler2D colorTexture, vec2 colorTexCoord){
    return texture(colorTexture, colorTexCoord);
}




vec4 combineColor0(vec4 destColor, vec4 srcColor, int op){
    switch(op){
    case 0x1 :
        return vec4(srcColor . rgb, srcColor . a * destColor . a);
    case 0x2 :
        return vec4(destColor . rgb, srcColor . a * destColor . a);
    }
    return destColor;
}








float filterTextSample1Tap(float offset, sampler2D colorTexture, vec2 colorTexCoord){
    return texture(colorTexture, colorTexCoord + vec2(offset, 0.0)). r;
}

























void filterTextSample9Tap(out vec4 outAlphaLeft,
                          out float outAlphaCenter,
                          out vec4 outAlphaRight,
                          sampler2D colorTexture,
                          vec2 colorTexCoord,
                          vec4 kernel,
                          float onePixel){
    bool wide = kernel . x > 0.0;
    outAlphaLeft =
        vec4(wide ? filterTextSample1Tap(- 4.0 * onePixel, colorTexture, colorTexCoord): 0.0,
             filterTextSample1Tap(- 3.0 * onePixel, colorTexture, colorTexCoord),
             filterTextSample1Tap(- 2.0 * onePixel, colorTexture, colorTexCoord),
             filterTextSample1Tap(- 1.0 * onePixel, colorTexture, colorTexCoord));
    outAlphaCenter = filterTextSample1Tap(0.0, colorTexture, colorTexCoord);
    outAlphaRight =
        vec4(filterTextSample1Tap(1.0 * onePixel, colorTexture, colorTexCoord),
             filterTextSample1Tap(2.0 * onePixel, colorTexture, colorTexCoord),
             filterTextSample1Tap(3.0 * onePixel, colorTexture, colorTexCoord),
             wide ? filterTextSample1Tap(4.0 * onePixel, colorTexture, colorTexCoord): 0.0);
}


float filterTextConvolve7Tap(vec4 alpha0, vec3 alpha1, vec4 kernel){
    return dot(alpha0, kernel)+ dot(alpha1, kernel . zyx);
}






float filterTextGammaCorrectChannel(float bgColor, float fgColor, sampler2D gammaLUT){
    return texture(gammaLUT, vec2(fgColor, 1.0 - bgColor)). r;
}










vec3 filterTextGammaCorrect(vec3 bgColor, vec3 fgColor, sampler2D gammaLUT){
    return vec3(filterTextGammaCorrectChannel(bgColor . r, fgColor . r, gammaLUT),
                filterTextGammaCorrectChannel(bgColor . g, fgColor . g, gammaLUT),
                filterTextGammaCorrectChannel(bgColor . b, fgColor . b, gammaLUT));
}





















































vec4 filterText(vec2 colorTexCoord,
                sampler2D colorTexture,
                sampler2D gammaLUT,
                vec2 colorTextureSize,
                vec4 filterParams0,
                vec4 filterParams1,
                vec4 filterParams2){

    vec4 kernel = filterParams0;
    vec3 bgColor = filterParams1 . rgb;
    vec3 fgColor = filterParams2 . rgb;
    bool gammaCorrectionEnabled = filterParams2 . a != 0.0;


    vec3 alpha;
    if(kernel . w == 0.0){
        alpha = texture(colorTexture, colorTexCoord). rrr;
    } else {
        vec4 alphaLeft, alphaRight;
        float alphaCenter;
        filterTextSample9Tap(alphaLeft,
                             alphaCenter,
                             alphaRight,
                             colorTexture,
                             colorTexCoord,
                             kernel,
                             1.0 / colorTextureSize . x);

        float r = filterTextConvolve7Tap(alphaLeft, vec3(alphaCenter, alphaRight . xy), kernel);
        float g = filterTextConvolve7Tap(vec4(alphaLeft . yzw, alphaCenter), alphaRight . xyz, kernel);
        float b = filterTextConvolve7Tap(vec4(alphaLeft . zw, alphaCenter, alphaRight . x),
                                         alphaRight . yzw,
                                         kernel);

        alpha = vec3(r, g, b);
    }


    if(gammaCorrectionEnabled)
        alpha = filterTextGammaCorrect(bgColor, alpha, gammaLUT);


    return vec4(mix(bgColor, fgColor, alpha), 1.0);
}



























































































































vec4 filterRadialGradient(vec2 colorTexCoord,
                          sampler2D colorTexture,
                          vec2 colorTextureSize,
                          vec2 fragCoord,
                          vec2 framebufferSize,
                          vec4 filterParams0,
                          vec4 filterParams1){
    vec2 lineFrom = filterParams0 . xy, lineVector = filterParams0 . zw;
    vec2 radii = filterParams1 . xy, uvOrigin = filterParams1 . zw;

    vec2 dP = colorTexCoord - lineFrom, dC = lineVector;
    float dR = radii . y - radii . x;

    float a = dot(dC, dC)- dR * dR;
    float b = dot(dP, dC)+ radii . x * dR;
    float c = dot(dP, dP)- radii . x * radii . x;
    float discrim = b * b - a * c;

    vec4 color = vec4(0.0);
    if(abs(discrim)>= 0.00001){
        vec2 ts = vec2(sqrt(discrim)* vec2(1.0, - 1.0)+ vec2(b))/ vec2(a);
        if(ts . x > ts . y)
            ts = ts . yx;
        float t = ts . x >= 0.0 ? ts . x : ts . y;
        color = texture(colorTexture, uvOrigin + vec2(clamp(t, 0.0, 1.0), 0.0));
    }

    return color;
}
















































vec4 filterBlur(vec2 colorTexCoord,
                sampler2D colorTexture,
                vec2 colorTextureSize,
                vec4 filterParams0,
                vec4 filterParams1){

    vec2 srcOffsetScale = filterParams0 . xy / colorTextureSize;
    int support = int(filterParams0 . z);
    vec3 gaussCoeff = filterParams1 . xyz;


    float gaussSum = gaussCoeff . x;
    vec4 color = texture(colorTexture, colorTexCoord)* gaussCoeff . x;
    gaussCoeff . xy *= gaussCoeff . yz;









    for(int i = 1;i <= support;i += 2){
        float gaussPartialSum = gaussCoeff . x;
        gaussCoeff . xy *= gaussCoeff . yz;
        gaussPartialSum += gaussCoeff . x;

        vec2 srcOffset = srcOffsetScale *(float(i)+ gaussCoeff . x / gaussPartialSum);
        color +=(texture(colorTexture, colorTexCoord - srcOffset)+
                  texture(colorTexture, colorTexCoord + srcOffset))* gaussPartialSum;

        gaussSum += 2.0 * gaussPartialSum;
        gaussCoeff . xy *= gaussCoeff . yz;
    }


    return color / gaussSum;
}







vec4 filterNone(vec2 colorTexCoord, sampler2D colorTexture){
    return sampleColor(colorTexture, colorTexCoord);
}








































vec4 filterColor(vec2 colorTexCoord,
                 sampler2D colorTexture,
                 sampler2D gammaLUT,
                 vec2 colorTextureSize,
                 vec2 fragCoord,
                 vec2 framebufferSize,
                 vec4 filterParams0,
                 vec4 filterParams1,
                 vec4 filterParams2,
                 int colorFilter){
    switch(colorFilter){
    case 0x1 :
        return filterRadialGradient(colorTexCoord,
                                    colorTexture,
                                    colorTextureSize,
                                    fragCoord,
                                    framebufferSize,
                                    filterParams0,
                                    filterParams1);
    case 0x3 :
        return filterBlur(colorTexCoord,
                          colorTexture,
                          colorTextureSize,
                          filterParams0,
                          filterParams1);
    case 0x2 :
        return filterText(colorTexCoord,
                          colorTexture,
                          gammaLUT,
                          colorTextureSize,
                          filterParams0,
                          filterParams1,
                          filterParams2);
    }
    return filterNone(colorTexCoord, colorTexture);
}




vec3 compositeSelect(bvec3 cond, vec3 ifTrue, vec3 ifFalse){
    return vec3(cond . x ? ifTrue . x : ifFalse . x,
                cond . y ? ifTrue . y : ifFalse . y,
                cond . z ? ifTrue . z : ifFalse . z);
}

float compositeDivide(float num, float denom){
    return denom != 0.0 ? num / denom : 0.0;
}

vec3 compositeColorDodge(vec3 destColor, vec3 srcColor){
    bvec3 destZero = equal(destColor, vec3(0.0)), srcOne = equal(srcColor, vec3(1.0));
    return compositeSelect(destZero,
                           vec3(0.0),
                           compositeSelect(srcOne, vec3(1.0), destColor /(vec3(1.0)- srcColor)));
}


vec3 compositeHSLToRGB(vec3 hsl){
    float a = hsl . y * min(hsl . z, 1.0 - hsl . z);
    vec3 ks = mod(vec3(0.0, 8.0, 4.0)+ vec3(hsl . x * 1.9098593171027443), 12.0);
    return hsl . zzz - clamp(min(ks - vec3(3.0), vec3(9.0)- ks), - 1.0, 1.0)* a;
}


vec3 compositeRGBToHSL(vec3 rgb){
    float v = max(max(rgb . r, rgb . g), rgb . b), xMin = min(min(rgb . r, rgb . g), rgb . b);
    float c = v - xMin, l = mix(xMin, v, 0.5);
    vec3 terms = rgb . r == v ? vec3(0.0, rgb . gb):
                 rgb . g == v ? vec3(2.0, rgb . br):
                              vec3(4.0, rgb . rg);
    float h = 1.0471975511965976 * compositeDivide(terms . x * c + terms . y - terms . z, c);
    float s = compositeDivide(c, v);
    return vec3(h, s, l);
}

vec3 compositeScreen(vec3 destColor, vec3 srcColor){
    return destColor + srcColor - destColor * srcColor;
}

vec3 compositeHardLight(vec3 destColor, vec3 srcColor){
    return compositeSelect(lessThanEqual(srcColor, vec3(0.5)),
                           destColor * vec3(2.0)* srcColor,
                           compositeScreen(destColor, vec3(2.0)* srcColor - vec3(1.0)));
}

vec3 compositeSoftLight(vec3 destColor, vec3 srcColor){
    vec3 darkenedDestColor =
        compositeSelect(lessThanEqual(destColor, vec3(0.25)),
                        ((vec3(16.0)* destColor - 12.0)* destColor + 4.0)* destColor,
                        sqrt(destColor));
    vec3 factor = compositeSelect(lessThanEqual(srcColor, vec3(0.5)),
                                  destColor *(vec3(1.0)- destColor),
                                  darkenedDestColor - destColor);
    return destColor +(srcColor * 2.0 - 1.0)* factor;
}

vec3 compositeHSL(vec3 destColor, vec3 srcColor, int op){
    switch(op){
    case 0xc :
        return vec3(srcColor . x, destColor . y, destColor . z);
    case 0xd :
        return vec3(destColor . x, srcColor . y, destColor . z);
    case 0xe :
        return vec3(srcColor . x, srcColor . y, destColor . z);
    default :
        return vec3(destColor . x, destColor . y, srcColor . z);
    }
}

vec3 compositeRGB(vec3 destColor, vec3 srcColor, int op){
    switch(op){
    case 0x1 :
        return destColor * srcColor;
    case 0x2 :
        return compositeScreen(destColor, srcColor);
    case 0x3 :
        return compositeHardLight(srcColor, destColor);
    case 0x4 :
        return min(destColor, srcColor);
    case 0x5 :
        return max(destColor, srcColor);
    case 0x6 :
        return compositeColorDodge(destColor, srcColor);
    case 0x7 :
        return vec3(1.0)- compositeColorDodge(vec3(1.0)- destColor, vec3(1.0)- srcColor);
    case 0x8 :
        return compositeHardLight(destColor, srcColor);
    case 0x9 :
        return compositeSoftLight(destColor, srcColor);
    case 0xa :
        return abs(destColor - srcColor);
    case 0xb :
        return destColor + srcColor - vec3(2.0)* destColor * srcColor;
    case 0xc :
    case 0xd :
    case 0xe :
    case 0xf :
        return compositeHSLToRGB(compositeHSL(compositeRGBToHSL(destColor),
                                              compositeRGBToHSL(srcColor),
                                              op));
    }
    return srcColor;
}




















vec4 composite(vec4 srcColor,
               sampler2D destTexture,
               vec2 destTextureSize,
               vec2 fragCoord,
               int op){
    if(op == 0x0)
        return srcColor;


    vec2 destTexCoord = fragCoord / destTextureSize;
    vec4 destColor = texture(destTexture, destTexCoord);
    vec3 blendedRGB = compositeRGB(destColor . rgb, srcColor . rgb, op);
    return vec4(srcColor . a *(1.0 - destColor . a)* srcColor . rgb +
                srcColor . a * destColor . a * blendedRGB +
                (1.0 - srcColor . a)* destColor . rgb,
                1.0);
}


















float sampleMask(float maskAlpha,
                 sampler2D maskTexture,
                 vec3 maskTexCoord,
                 int maskCtrl){
    if(maskCtrl == 0)
        return maskAlpha;
    float coverage = texture(maskTexture, maskTexCoord . xy). r + maskTexCoord . z;
    if((maskCtrl & 0x1)!= 0)
        coverage = abs(coverage);
    else
        coverage = 1.0 - abs(1.0 - mod(coverage, 2.0));
    return min(maskAlpha, coverage);
}




void calculateColor(int tileCtrl, int ctrl){

    int maskCtrl0 =(tileCtrl >> 0)& 0x3;
    float maskAlpha = 1.0;
    maskAlpha = sampleMask(maskAlpha, uMaskTexture0, vMaskTexCoord0, maskCtrl0);


    vec4 color = vBaseColor;
    int color0Combine =(ctrl >> 6)&
                                       0x3;
    if(color0Combine != 0){
        int color0Filter =(ctrl >> 4)& 0x3;
        vec4 color0 = filterColor(vColorTexCoord0,
                                  uColorTexture0,
                                  uGammaLUT,
                                  uColorTexture0Size,
                                  gl_FragCoord . xy,
                                  uFramebufferSize,
                                  uFilterParams0,
                                  uFilterParams1,
                                  uFilterParams2,
                                  color0Filter);
        color = combineColor0(color, color0, color0Combine);
    }


    color . a *= maskAlpha;


    int compositeOp =(ctrl >> 8)& 0xf;
    color = composite(color, uDestTexture, uFramebufferSize, gl_FragCoord . xy, compositeOp);


    color . rgb *= color . a;
    oFragColor = color;
}





void main(){
    calculateColor(int(vTileCtrl), uCtrl);
}


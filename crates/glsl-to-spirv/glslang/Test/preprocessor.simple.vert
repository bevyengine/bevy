#version 310 es
#define X 1
#define Y clamp
#define Z X

#define F 1, 2

#define make_function \
  float fn ( float x ) \
  {\
    return x + 4.0; \
  }

make_function

int main() {
  gl_Position = vec4(X);
  gl_Position = Y(1, 2, 3);
  gl_Position = vec4(Z);
  gl_Position = vec4(F);
  gl_Position = vec4(fn(3));
  [] . ++ --
  + - * % / - ! ~
  << >> < > <= >=
  == !=
  & ^ | && ^^ || ? :
  += -= *= /= %= <<= >>= &= |= ^=
  1.2 2E10 5u -5lf
}

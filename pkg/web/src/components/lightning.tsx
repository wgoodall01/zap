import React, { useRef, useEffect } from "react";

// Originally adapted from:
// https://reactbits.dev/backgrounds/lightning

// --- WebGL Shader Definitions (moved outside component for clarity) ---
const vertexShaderSource = `
  attribute vec2 aPosition;
  void main() {
    gl_Position = vec4(aPosition, 0.0, 1.0);
  }
`;

const fragmentShaderSource = `
  precision mediump float;
  uniform vec2 iResolution;
  uniform float iTime;
  uniform float uHue;
  uniform float uXOffset;
  uniform float uSpeed;
  uniform float uIntensity;
  uniform float uSize;
  // ... (rest of your GLSL code for noise, fbm, hsv2rgb, mainImage) ...
  #define OCTAVE_COUNT 10

  vec3 hsv2rgb(vec3 c) {
      vec3 rgb = clamp(abs(mod(c.x * 6.0 + vec3(0.0,4.0,2.0), 6.0) - 3.0) - 1.0, 0.0, 1.0);
      return c.z * mix(vec3(1.0), rgb, c.y);
  }

  float hash11(float p) {
      p = fract(p * .1031);
      p *= p + 33.33;
      p *= p + p;
      return fract(p);
  }

  float hash12(vec2 p) {
      vec3 p3 = fract(vec3(p.xyx) * .1031);
      p3 += dot(p3, p3.yzx + 33.33);
      return fract((p3.x + p3.y) * p3.z);
  }

  mat2 rotate2d(float theta) {
      float c = cos(theta);
      float s = sin(theta);
      return mat2(c, -s, s, c);
  }

  float noise(vec2 p) {
      vec2 ip = floor(p);
      vec2 fp = fract(p);
      float a = hash12(ip);
      float b = hash12(ip + vec2(1.0, 0.0));
      float c = hash12(ip + vec2(0.0, 1.0));
      float d = hash12(ip + vec2(1.0, 1.0));
      vec2 t = smoothstep(0.0, 1.0, fp);
      return mix(mix(a, b, t.x), mix(c, d, t.x), t.y);
  }

  float fbm(vec2 p) {
      float value = 0.0;
      float amplitude = 0.5;
      for (int i = 0; i < OCTAVE_COUNT; ++i) {
          value += amplitude * noise(p);
          p *= rotate2d(0.45);
          p *= 2.0;
          amplitude *= 0.5;
      }
      return value;
  }

  void mainImage( out vec4 fragColor, in vec2 fragCoord ) {
      vec2 uv = fragCoord / iResolution.xy;
      uv = 2.0 * uv - 1.0;
      uv.x *= iResolution.x / iResolution.y;
      uv.x += uXOffset;
      uv += 2.0 * fbm(uv * uSize + 0.8 * iTime * uSpeed) - 1.0;
      float dist = abs(uv.x);
      vec3 baseColor = hsv2rgb(vec3(uHue / 360.0, 0.7, 0.8));
      vec3 col = baseColor * pow(mix(0.0, 0.07, hash11(iTime * uSpeed)) / dist, 1.0) * uIntensity;
      col = pow(col, vec3(1.0));
      fragColor = vec4(col, 1.0);
  }

  void main() {
      mainImage(gl_FragColor, gl_FragCoord.xy);
  }
`;

export const Lightning = ({
  hue = 230,
  xOffset = 0,
  speed = 1,
  intensity = 1,
  size = 1,
  ...rest
}: {
  hue?: number;
  xOffset?: number;
  speed?: number;
  intensity?: number;
  size?: number;
} & React.HTMLAttributes<HTMLCanvasElement>) => {
  const canvasRef = useRef(null);
  // --- Refs to hold persistent values (WebGL context, animation state, props) ---
  const glRef = useRef(null);
  const animationFrameRef = useRef(null);
  const propsRef = useRef({ hue, xOffset, speed, intensity, size });
  const uniformLocationsRef = useRef({});
  const programRef = useRef(null);
  const bufferRef = useRef(null);
  const shaderRefs = useRef([]);

  // --- Update props ref on every render ---
  // This ensures the render loop always has access to the latest props.
  useEffect(() => {
    propsRef.current = { hue, xOffset, speed, intensity, size };
  }, [hue, xOffset, speed, intensity, size]);

  // --- Initialization and Render Loop Effect (runs only once on mount) ---
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const gl = canvas.getContext("webgl");
    if (!gl) {
      console.error("WebGL not supported");
      return;
    }
    glRef.current = gl;

    // --- WebGL Helper Functions ---
    const compileShader = (source, type) => {
      const shader = gl.createShader(type);
      if (!shader) return null;
      gl.shaderSource(shader, source);
      gl.compileShader(shader);
      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.error("Shader compile error:", gl.getShaderInfoLog(shader));
        gl.deleteShader(shader);
        return null;
      }
      shaderRefs.current.push(shader); // Store shader for cleanup
      return shader;
    };

    const createProgram = (vertexShader, fragmentShader) => {
      const program = gl.createProgram();
      if (!program) return null;
      gl.attachShader(program, vertexShader);
      gl.attachShader(program, fragmentShader);
      gl.linkProgram(program);
      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
        console.error("Program linking error:", gl.getProgramInfoLog(program));
        gl.deleteProgram(program);
        return null;
      }
      return program;
    };

    // --- One-Time Setup ---
    const vertexShader = compileShader(vertexShaderSource, gl.VERTEX_SHADER);
    const fragmentShader = compileShader(
      fragmentShaderSource,
      gl.FRAGMENT_SHADER,
    );
    if (!vertexShader || !fragmentShader) return;

    const program = createProgram(vertexShader, fragmentShader);
    if (!program) return;
    programRef.current = program;
    gl.useProgram(program);

    // Buffer setup
    const vertices = new Float32Array([
      -1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1,
    ]);
    const vertexBuffer = gl.createBuffer();
    bufferRef.current = vertexBuffer;
    gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);
    const aPosition = gl.getAttribLocation(program, "aPosition");
    gl.enableVertexAttribArray(aPosition);
    gl.vertexAttribPointer(aPosition, 2, gl.FLOAT, false, 0, 0);

    // Get uniform locations once and store them in a ref
    uniformLocationsRef.current = {
      iResolution: gl.getUniformLocation(program, "iResolution"),
      iTime: gl.getUniformLocation(program, "iTime"),
      uHue: gl.getUniformLocation(program, "uHue"),
      uXOffset: gl.getUniformLocation(program, "uXOffset"),
      uSpeed: gl.getUniformLocation(program, "uSpeed"),
      uIntensity: gl.getUniformLocation(program, "uIntensity"),
      uSize: gl.getUniformLocation(program, "uSize"),
    };

    // --- Resize Handling ---
    const resizeCanvas = () => {
      canvas.width = canvas.clientWidth;
      canvas.height = canvas.clientHeight;
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.uniform2f(
        uniformLocationsRef.current.iResolution,
        canvas.width,
        canvas.height,
      );
    };
    resizeCanvas();
    window.addEventListener("resize", resizeCanvas);

    // --- Render Loop ---
    const startTime = performance.now();
    const render = () => {
      const { hue, xOffset, speed, intensity, size } = propsRef.current;
      const locations = uniformLocationsRef.current;

      // Update time uniform
      gl.uniform1f(locations.iTime, (performance.now() - startTime) / 1000.0);

      // Update uniforms based on current props from ref
      gl.uniform1f(locations.uHue, hue);
      gl.uniform1f(locations.uXOffset, xOffset);
      gl.uniform1f(locations.uSpeed, speed);
      gl.uniform1f(locations.uIntensity, intensity);
      gl.uniform1f(locations.uSize, size);

      gl.drawArrays(gl.TRIANGLES, 0, 6);
      animationFrameRef.current = requestAnimationFrame(render);
    };
    render(); // Start the loop

    // --- Cleanup Function ---
    return () => {
      window.removeEventListener("resize", resizeCanvas);
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      if (glRef.current) {
        // Detach and delete shaders
        shaderRefs.current.forEach((shader) => {
          if (programRef.current) {
            glRef.current.detachShader(programRef.current, shader);
          }
          glRef.current.deleteShader(shader);
        });
        glRef.current.deleteProgram(programRef.current);
        glRef.current.deleteBuffer(bufferRef.current);
      }
    };
  }, []); // Empty dependency array ensures this runs only once on mount

  return <canvas {...rest} ref={canvasRef} />;
};

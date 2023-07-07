struct Globals {
    viewport: vec2<f32>,
    scale: f32,
    tick: u32,
}

@group(0) @binding(0)
var<uniform> globals: Globals;

// Vertex shader
//------------------------------------------------------------------------------

struct VertexInput {
    @location(0) index: u32,
    @location(1) rect: vec4<f32>,
    @location(2) fill: u32,
    @location(3) tex_rect: vec4<f32>,
    @location(4) tex_fill: u32,
    @location(5) rotation_base: f32,
    @location(6) rotation_rate: f32,
    @location(7) rotation_origin: vec2<f32>,
    @location(8) border_radius: vec2<u32>,
    @location(9) border_size: u32,
    @location(10) border_color: vec4<u32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) bg_fill: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) tex_area: f32,
    @location(3) tex_fill: vec4<f32>,
    @location(4) rect: vec4<f32>,
    @location(5) angle: f32,
    @location(6) rot_origin: vec2<f32>,
    @location(7) border_size: vec4<f32>,
    @location(8) border_top_left_radius: vec2<f32>,
    @location(9) border_top_right_radius: vec2<f32>,
    @location(10) border_bottom_right_radius: vec2<f32>,
    @location(11) border_bottom_left_radius: vec2<f32>,    
    @location(12) border_color_t: vec4<f32>,
    @location(13) border_color_b: vec4<f32>,
    @location(14) border_color_l: vec4<f32>,
    @location(15) border_color_r: vec4<f32>,
}


@vertex
fn vs_main(in: VertexInput, @builtin(vertex_index) n: u32) -> VertexOutput {
    // let i = n % 6u;
    let i = in.index;
    let tick = f32(globals.tick);
    // let tick = 0.;
    let viewport_size = floor(globals.viewport / globals.scale);

    // Calc vertex output coords
    var vec_pos: vec2<f32>;
    var tex_pos: vec2<f32>;

    let vx0 = in.rect.x;
    let vy0 = in.rect.y;
    let vx1 = in.rect.z + vx0;
    let vy1 = in.rect.w + vy0;

    let tx0 = in.tex_rect.x;
    let ty0 = in.tex_rect.y;
    let tx1 = in.tex_rect.z + tx0;
    let ty1 = in.tex_rect.w + ty0;

    if i == 0u || i == 5u { // br;
        vec_pos.x = vx1;
        vec_pos.y = vy1;
        tex_pos.x = tx1;
        tex_pos.y = ty1;
    }
    if i == 1u { // tr
        vec_pos.x = vx1;
        vec_pos.y = vy0;
        tex_pos.x = tx1;
        tex_pos.y = ty0;
    }
    if i == 2u || i == 3u { // tl
        vec_pos.x = vx0;
        vec_pos.y = vy0;
        tex_pos.x = tx0;
        tex_pos.y = ty0;
    }
    if i == 4u { // bl
        vec_pos.x = vx0;
        vec_pos.y = vy1;
        tex_pos.x = tx0;
        tex_pos.y = ty1;
    }
    vec_pos = floor(vec_pos);
    tex_pos = floor(tex_pos);

    // border-radius can't be larger than 50% of the rect w/h
    let brx = to_vec4_f32(in.border_radius.x).x;
    let bry = to_vec4_f32(in.border_radius.y).x;
    var br = vec2<f32>(
        min(brx, in.rect.z / 2.),
        min(bry, in.rect.w / 2.),
        // min(in.border_radius.x, in.rect.z / 2.),
        // min(in.border_radius.y, in.rect.w / 2.),
    );

    // Apply rotation
    let angle = in.rotation_base + (in.rotation_rate * tick);
    var pos = in.rect.xy;
    let size = in.rect.zw;
    var rot_origin = pos + size / 2.0;
    if angle != 0. {
        let rot_mat = mat2x2<f32>(
            cos(angle),
            -sin(angle),
            sin(angle),
            cos(angle)
        );
        vec_pos -= rot_origin;
        vec_pos *= rot_mat;
        vec_pos += rot_origin;
    }
    // Convert xy to normalized device coordinates (NDC)
    vec_pos = (vec_pos / viewport_size) * 2. - 1.;

    var out: VertexOutput;
    out.pos = vec4<f32>(vec_pos * vec2<f32>(1., -1.), 0., 1.);
    out.tex_coords = tex_pos;
    out.bg_fill = to_rgba(in.fill);
    out.tex_fill = to_rgba(in.tex_fill);
    out.tex_area = in.tex_rect.z * in.tex_rect.w;
    out.border_top_left_radius = vec2<f32>(
        min(to_vec4_f32(in.border_radius.x).x, in.rect.z / 2.),
        min(to_vec4_f32(in.border_radius.y).x, in.rect.w / 2.),
    );
    out.border_top_right_radius = vec2<f32>(
        min(to_vec4_f32(in.border_radius.x).y, in.rect.z / 2.),
        min(to_vec4_f32(in.border_radius.y).y, in.rect.w / 2.),
    );
    out.border_bottom_right_radius = vec2<f32>(
        min(to_vec4_f32(in.border_radius.x).z, in.rect.z / 2.),
        min(to_vec4_f32(in.border_radius.y).z, in.rect.w / 2.),
    );
    out.border_bottom_left_radius = vec2<f32>(
        min(to_vec4_f32(in.border_radius.x).w, in.rect.z / 2.),
        min(to_vec4_f32(in.border_radius.y).w, in.rect.w / 2.),
    );
    out.rect = in.rect;
    out.angle = angle;
    out.rot_origin = rot_origin;
    out.border_size = to_vec4_f32(in.border_size);
    out.border_color_t = to_rgba(in.border_color.x);
    out.border_color_r = to_rgba(in.border_color.y);
    out.border_color_b = to_rgba(in.border_color.z);
    out.border_color_l = to_rgba(in.border_color.w);
    // out.border_color_t = vec4<f32>(0., 0., 1., 1.);
    // out.border_color_r = vec4<f32>(.5, 0., .5, 1.);
    // out.border_color_b = vec4<f32>(1., 0., .5, 1.);
    // out.border_color_l = vec4<f32>(1., 0., 0., 1.);
    return out;
}

fn to_rgba(color: u32) -> vec4<f32> {

    let r = f32(((color >> 0u) & 0xFFu)) / 255.;
    let g = f32(((color >> 8u) & 0xFFu)) / 255.;
    let b = f32(((color >> 16u) & 0xFFu)) / 255.;
    let a = f32(((color >> 24u) & 0xFFu)) / 255.;
    return vec4<f32>(r, g, b, a);
    // if color == 0x9900ffffu {
    //     // return vec4<f32>(1., 1., 0., .666666);
    //     return vec4<f32>(1., 1., 1., .666666);
    // }
    // return to_vec4_f32(color) / 1.;
}

fn to_vec4_f32(color: u32) -> vec4<f32> {
    let x = f32(((color >> 0u) & 0xFFu));
    let y = f32(((color >> 8u) & 0xFFu));
    let z = f32(((color >> 16u) & 0xFFu));
    let w = f32(((color >> 24u) & 0xFFu));
    return vec4<f32>(x, y, z, w);
}
 

// Fragment shader
//------------------------------------------------------------------------------
// TODO: explore texture-arrays: https://github.com/gfx-rs/wgpu/blob/trunk/wgpu/examples/texture-arrays/main.rs

@group(1) @binding(0)
var t_spritesheet: texture_2d<f32>;

@group(1) @binding(1)
var s_spritesheet: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_dims = textureDimensions(t_spritesheet);
    let tex_dimsf = vec2<f32>(tex_dims.xy);
    let uv = in.tex_coords.xy / tex_dimsf;
    let tex_color = textureSample(t_spritesheet, s_spritesheet, uv);
    
    // Unapply transformation so we can do calculations easier
    let rot_mat = mat2x2<f32>(
        cos(in.angle),
        sin(in.angle),
        -sin(in.angle),
        cos(in.angle)
    );
    var pos = in.pos.xy;
    let rot_origin = in.rot_origin;
    pos -= rot_origin;
    pos *= rot_mat;
    pos += rot_origin;

    let px = pos.x;
    let py = pos.y;
    let rx = in.rect.x;
    let ry = in.rect.y;
    let rw = in.rect.z;
    let rh = in.rect.w;

    let bst = in.border_size.x; // top
    let bsr = in.border_size.y; // right
    let bsb = in.border_size.z; // bottom
    let bsl = in.border_size.w; // left

    let black = vec4<f32>(0., 0., 0., 1.);


    // Border radius (top-left)
    let brtl = in.border_top_left_radius;
    if brtl.x > 0. || brtl.y > 0. {
        var e: vec4<f32>;
        e.z = brtl.x * 2.;
        e.w = brtl.y * 2.;
        e.x = rx;
        e.y = ry;
        var r: vec4<f32>;
        r.z = e.z * .5;
        r.w = e.w * .5;
        r.x = e.x;
        r.y = e.y;
        if intersects_rect(px, py, r.x, r.y, r.z, r.w) {
            if !intersects_ellipse(px, py, e.x, e.y, e.z, e.w) {
                    discard;
            }
            if ellipse_edge_distance(px, py, e.x, e.y, e.z, e.w) <= bst { // top
                if intersects_rect(px, py, r.x, r.y, r.z, r.w * .5) {
                    return in.border_color_t;
                }
            }
            if ellipse_edge_distance(px, py, e.x, e.y, e.z, e.w) <= bsl { // left
                if intersects_rect(px, py, r.x, r.y, r.z * .5, r.w) {
                    return in.border_color_l;
                }
            }
        }
    }
        
    // Border radius (top-right)
    let brtr = in.border_top_right_radius;
    if brtr.x > 0. || brtr.y > 0. {
        var e: vec4<f32>;
        e.z = brtr.x * 2.;
        e.w = brtr.y * 2.;
        e.x = rx + rw + -e.z;
        e.y = ry;
        var r: vec4<f32>;
        r.z = e.z * .5;
        r.w = e.w * .5;
        r.x = e.x + r.z;
        r.y = e.y;
        if intersects_rect(px, py, r.x, r.y, r.z, r.w) {
            if !intersects_ellipse(px, py, e.x, e.y, e.z, e.w) {
                    discard;
            }
            if ellipse_edge_distance(px, py, e.x, e.y, e.z, e.w) <= bst { // top
                if intersects_rect(px, py, r.x, r.y, r.z * .5, r.w) {
                    return in.border_color_t;
                }
            }
            if ellipse_edge_distance(px, py, e.x, e.y, e.z, e.w) <= bsr { // right
                if intersects_rect(px, py, r.x + r.z * .5, r.y, r.z * .5, r.w) {
                    return in.border_color_r;
                }
            }
        }
    }

    // Border radius (bottom-right)
    let brbr = in.border_bottom_right_radius;
    if brbr.x > 0. || brbr.y > 0. {
        var e: vec4<f32>;
        e.z = brbr.x * 2.;
        e.w = brbr.y * 2.;
        e.x = rx + rw + -e.z;
        e.y = ry + rh + -e.w;
        var r: vec4<f32>;
        r.z = e.z * .5;
        r.w = e.w * .5;
        r.x = e.x + r.z;
        r.y = e.y + r.w;
        if intersects_rect(px, py, r.x, r.y, r.z, r.w) {
            if !intersects_ellipse(px, py, e.x, e.y, e.z, e.w) {
                    discard;
            }
            if ellipse_edge_distance(px, py, e.x, e.y, e.z, e.w) <= bst { // bottom
                if intersects_rect(px, py, r.x, r.y + r.w * .5, r.z, r.w * .5) {
                    return in.border_color_b;
                }
            }
            if ellipse_edge_distance(px, py, e.x, e.y, e.z, e.w) <= bsr { // right
                if intersects_rect(px, py, r.x + r.z * .5, r.y, r.z * .5, r.w) {
                    return in.border_color_r;
                }
            }
        }
    }

    // Border radius (bottom-left)
    let brbl = in.border_bottom_left_radius;
    if brbl.x > 0. || brbl.y > 0. {
        var e: vec4<f32>;
        e.z = brbl.x * 2.;
        e.w = brbl.y * 2.;
        e.x = rx;
        e.y = ry + rh + -e.w;
        var r: vec4<f32>;
        r.z = e.z * .5;
        r.w = e.w * .5;
        r.x = e.x;
        r.y = e.y + r.w;
        if intersects_rect(px, py, r.x, r.y, r.z, r.w) {
            if !intersects_ellipse(px, py, e.x, e.y, e.z, e.w) {
                    discard;
            }
            if ellipse_edge_distance(px, py, e.x, e.y, e.z, e.w) <= bst { // bottom
                if intersects_rect(px, py, r.x, r.y + r.w * .5, r.z, r.w * .5) {
                    return in.border_color_b;
                }
            }
            if ellipse_edge_distance(px, py, e.x, e.y, e.z, e.w) <= bsl { // left
                if intersects_rect(px, py, r.x, r.y, r.z * .5, r.w) {
                    return in.border_color_l;
                }
            }
        }
    }

    // Border color (top)
    if intersects_rect(px, py, rx, ry, rw, bst) {
        return in.border_color_t;
    }

    // Border color (right)
    if intersects_rect(px, py, (rx + rw) - bsr, ry, bsr, rh) {
        return in.border_color_r;
    }

    // Border color (bottom)
    if intersects_rect(px, py, rx, (ry + rh) - bsb, rw, bsb) {
        return in.border_color_b;
    }

    // Border color (left)
    if intersects_rect(px, py, rx, ry, bsl, rh) {
        return in.border_color_l;
    }
    
    // Get the fill color
    var fill = tex_color;
    // fg color
    if in.tex_fill.a > 0. && tex_color.a > 0. {
        fill = in.tex_fill;
    }
    // bg color
    if in.bg_fill.a > 0. && tex_color.a == 0. {
        fill = in.bg_fill;
    }
    fill.a = ceil(fill.a); // Only 0 or 1 opacity supported
    return fill;

    // // If there's no sampled texture color, use the background fill
    // if tex_color.a == 0. {
    //     // let rgb = in.bg_fill.rgb;
    //     // FIXME: alpha channel weirdness
    //     // Any alpha value > ~.75 seems to effectively be 1. ¯\_(ツ)_/¯
    //     // no visual difference if the following statement is uncommented
    //     // if in.bg_fill.a == 1. {
    //     //     return in.bg_fill;
    //     // }
    //     // let rgba = vec4<f32>(rgb, in.bg_fill.a * .5);
    //     // let rgba = vec4<f32>(rgb, in.bg_fill.a);
    //     // return rgba;
    //     // if in.bg_fill.a > .6 && in.bg_fill.a < .7 {
    //     //     return vec4<f32>(1., 1., 1., 1.);
    //     // }
    //     // return in.bg_fill;
    // }

    // // Blend the sample texture color with the input texture fill color
    // let rgb = mix(tex_color.rgb, in.tex_fill.rgb, in.tex_fill.a);
    // let rgba = vec4<f32>(rgb, tex_color.a);
    // return rgba;
}

// The equation of an ellipse centered at the point (h, k) with semi-major axis 'a' and semi-minor axis 'b' is:
// ((x - h)^2 / a^2) + ((y - k)^2 / b^2) <= 1
fn intersects_ellipse(pointX: f32, pointY: f32, topLeftX: f32, topLeftY: f32, width: f32, height: f32) -> bool {
    let radiusX = width / 2.;
    let radiusY = height / 2.;

    let centerX = topLeftX + radiusX;
    let centerY = topLeftY + radiusY;

    let distanceX = pointX - centerX;
    let distanceY = pointY - centerY;

    return ((distanceX * distanceX) / (radiusX * radiusX) + (distanceY * distanceY) / (radiusY * radiusY)) < 1.;
}

fn ellipse_edge_distance(pointX: f32, pointY: f32, topLeftX: f32, topLeftY: f32, width: f32, height: f32) -> f32 {
    let radiusX = width / 2.;
    let radiusY = height / 2.;

    let centerX = topLeftX + radiusX;
    let centerY = topLeftY + radiusY;

    var dx = pointX - centerX;
    var dy = pointY - centerY;

    var angle = atan2(dy * radiusX, dx * radiusY);
    var ellipseX = radiusX * cos(angle);
    var ellipseY = radiusY * sin(angle);

    var distanceX = abs(ellipseX - dx);
    var distanceY = abs(ellipseY - dy);

    var distance = sqrt(distanceX * distanceX + distanceY * distanceY);
    return distance;
}

fn intersects_rect(pointX: f32, pointY: f32, rectX: f32, rectY: f32, rectWidth: f32, rectHeight: f32) -> bool {
    return (pointX >= rectX && // Point is to the right of the left edge
    pointX <= rectX + rectWidth && // Point is to the left of the right edge
    pointY >= rectY && // Point is below the top edge
    pointY <= rectY + rectHeight); // Point is above the bottom edge
}

fn is_point_above_arc(topLeftX: f32, topLeftY: f32, borderRadiusX: f32, borderRadiusY: f32, pointX: f32, pointY: f32) -> bool {
  // Calculate the center of the top-left arc
    let arcCenterX = topLeftX + borderRadiusX;
    let arcCenterY = topLeftY + borderRadiusY;

  // Calculate the angle at the current point's x-coordinate
    let angle = acos((pointX - arcCenterX) / borderRadiusX);

  // Calculate the y-coordinate of the arc at the current point's x-coordinate
    let arcY = arcCenterY - borderRadiusY * sin(angle);

  // Check if the point is located above the edge of the arc
    let isAbove = pointY < arcY;

  // Check if the point is to the left of the middle x position
    let isLeft = pointX <= arcCenterX;

    return isAbove && isLeft;
}

fn arc_dist(topLeftX: f32, topLeftY: f32, borderRadiusX: f32, borderRadiusY: f32, pointX: f32, pointY: f32, flipY: bool) -> vec2<f32> {
    // Calculate the center of the top-left arc
    let arcCenterX = topLeftX + borderRadiusX;
    let arcCenterY = topLeftY + borderRadiusY;

    // Calculate the angle at the current point's x-coordinate
    let angle = acos((pointX - arcCenterX) / borderRadiusX);

    // Calculate the y-coordinate of the arc
    var a = sin(angle);
    if flipY {
        a *= -1.;
    }
    let arcY = arcCenterY - borderRadiusY * a;

    let distx = pointX - arcCenterX;
    let disty = pointY - arcY;

    return vec2<f32>(distx, disty);
}

fn is_top_left_of_arc(topLeftX: f32, topLeftY: f32, borderRadiusX: f32, borderRadiusY: f32, pointX: f32, pointY: f32) -> bool {
    // Calculate the center of the top-left arc
    let arcCenterX = topLeftX + borderRadiusX;
    let arcCenterY = topLeftY + borderRadiusY;

    // Check if the point is located to the top-left of the edge of the arc
    let isToTopLeft = pointX < arcCenterX && pointY < arcCenterY;

    return isToTopLeft;
}

fn ellipse_dist(cx: f32, cy: f32, rx: f32, ry: f32, px: f32, py: f32) -> f32 {
    // Calculate the distance between the point and the center of the ellipse
    let dx = px - cx;
    let dy = py - cy;

    // Calculate the angle between the center of the ellipse and the given point
    let angle = atan2(dy / ry, dx / rx);

    // Calculate the point on the ellipse's circumference based on the angle
    let ex = cx + rx * cos(angle);
    let ey = cy + ry * sin(angle);

    // Calculate the distance between the given point and the point on the ellipse
    let powx = pow(px - ex, 2.);
    let powy = pow(py - ey, 2.);
    let dist = sqrt(powx + powy);

    return dist;
}

fn discard_top_left_border_radius(br: vec2<f32>, px: f32, py: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    let dx = px - rx; // left distance
    let dy = py - ry; // top distance
    let cx = (dx - br.x) / br.x; // ellipse center x
    let cy = (dy - br.y) / br.y; // ellipse center y
    let distance = sqrt(cx * cx + cy * cy);
    let is_top = dy < br.y;
    let is_left = dx < br.x;
    let is_top_left = is_top && is_left;
    // return distance > max(br.x, br.y) && is_top_left;
    return distance > 1. && is_top_left;
}

fn discard_top_right_border_radius(br: vec2<f32>, px: f32, py: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    let dx = px - (rx + rw); // right distance
    let dy = py - ry; // top distance
    let cx = (dx + br.x) / br.x; // ellipse center x
    let cy = (dy - br.y) / br.y; // ellipse center y
    let distance = sqrt(cx * cx + cy * cy);
    let is_top = dy < br.y;
    let is_right = px > (rx + rw) - br.x;
    let is_top_right = is_top && is_right;
    // return distance > max(br.x, br.y) && is_top_right;
    return distance > 1. && is_top_right;
}

fn discard_bottom_left_border_radius(br: vec2<f32>, px: f32, py: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    let dx = px - rx; // left distance
    let dy = py - (ry + rh); // bottom distance
    let cx = (dx - br.x) / br.x; // ellipse center x
    let cy = (dy + br.y) / br.y; // ellipse center y
    let distance = sqrt(cx * cx + cy * cy);
    let is_bottom = py > (ry + rh) - br.y;
    let is_left = dx < br.x;
    let is_bottom_left = is_bottom && is_left;
    // return distance > max(br.x, br.y) && is_bottom_left;
    return distance > 1. && is_bottom_left;
}

fn discard_bottom_right_border_radius(br: vec2<f32>, px: f32, py: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    let dx = (px - (rx + rw)); // right distance
    let dy = (py - (ry + rh)); // bottom distance
    let cx = (dx + br.x) / br.x; // ellipse center x
    let cy = (dy + br.y) / br.y; // ellipse center y
    let distance = sqrt(cx * cx + cy * cy);
    let is_bottom = py > (ry + rh) - br.y;
    let is_right = px > (rx + rw) - br.x;
    let is_bottom_right = is_bottom && is_right;
    // return distance > max(br.x, br.y) && is_bottom_right;
    return distance > 1. && is_bottom_right;
}
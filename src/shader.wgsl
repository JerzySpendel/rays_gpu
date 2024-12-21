struct Ray {
    orig: vec3<f32>,
    dir: vec3<f32>,
    color: vec3<f32>,
    screen_x: u32,
    screen_y: u32,
}

struct Ball { // object type 1.0
    center: vec3<f32>,
    radius: f32,
    material: u32,
}

struct Triangle { // object type 2.0
    v0: vec3<f32>,
    v1: vec3<f32>,
    v2: vec3<f32>,
}


@group(0)
@binding(0)
var<storage, read_write> v_indices: array<Ray>; // this is used as both input and output for convenience

@group(0)
@binding(1)
var<storage> balls: array<Ball>;

@group(0)
@binding(2)
var<uniform> pixel_delta_u: vec3<f32>;

@group(0)
@binding(3)
var<uniform> pixel_delta_v: vec3<f32>;


@group(0)
@binding(4)
var<storage> triangles: array<Triangle>;

@group(1) @binding(0)
var noise_texture: texture_2d<f32>;


fn pcg(v: u32) -> u32 {
    var seed = (v ^ 61u) ^ (v >> 16u);
    seed *= 9u;
    seed = seed ^ (seed >> 4u);
    seed *= 0x27d4eb2du;
    seed = seed ^ (seed >> 15u);
    return seed;
}

fn prng (p: f32) -> f32 {
  return f32(pcg(u32(p))) / f32(0xffffffffu);
}

fn triangle_hit(ray: Ray, triangle_id: u32) -> vec4<f32> {
    let kEpsilon = 0.0001;
    let triangle: Triangle = triangles[triangle_id];
    let v0v1 = triangle.v1 - triangle.v0;
    let v0v2 = triangle.v2 - triangle.v0;
    let N = cross(v0v1, v0v2);
    let area2 = length(N);

    let invalid = vec4<f32>(-1.0);
    if abs(dot(N, ray.dir)) < kEpsilon {
        return invalid;
    }

    let d = -dot(N, triangle.v0);
    let t = -(dot(N, ray.orig) + d) / dot(N, ray.dir);

    if t < 0.0 {
        return invalid;
    }

    let P = ray.orig + ray.dir * t;

    let edge0 = triangle.v1 - triangle.v0;
    let vp0 = P - triangle.v0;
    var C: vec3<f32> = cross(edge0, vp0);
    if dot(N, C) < 0.0 {
        return invalid;
    }

    let edge1 = triangle.v2 - triangle.v1;
    let vp1 = P - triangle.v1;
    C = cross(edge1, vp1);
    if dot(N, C) < 0.0 {
        return invalid;
    }

    let edge2 = triangle.v0 - triangle.v2;
    let vp2 = P - triangle.v2;
    C = cross(edge2, vp2);
    if dot(N, C) < 0.0 {
        return invalid;
    }

    return vec4<f32>(t, N.x, N.y, N.z);
}

fn has_hit(ray: Ray) -> array<vec3<f32>, 2> {
    let init_max_t = f32(100000000);
    let min_t: f32 = 0.001;
    var max_t: f32 = init_max_t;
    var ball_index: i32;

    var response: vec3<f32> = vec3<f32>(init_max_t, -1., -1.); // vec3<f32>(t, material, :unused_variable_space:)
    var N: vec3<f32>;
    let ok = arrayLength(&balls);

    for (var i: i32 = 0; i < i32(arrayLength(&balls)); i = i + 1){
        let ball = balls[i];
        let oc: vec3<f32> = ray.orig - ball.center;
        let a = dot(ray.dir, ray.dir);
        let half_b = dot(oc, ray.dir);
        let c = dot(oc, oc) - ball.radius * ball.radius;
        let discr: f32 = half_b * half_b - a * c;
        if discr > 0.0 {

            var solution = (-half_b - sqrt(discr)) / a;
            if solution > min_t && solution < response.x {
                response.x = solution;
                response.y = f32(ball.material);
                N = normalize(ray.orig + solution * ray.dir - ball.center);
            }

            solution = (-half_b + sqrt(discr)) / a;
            if solution > min_t && solution < response.x {
                response.x = solution;
                response.y = f32(ball.material);
                N = normalize(ray.orig + solution * ray.dir - ball.center);
            }

        }
    }

    for(var i: i32 = 0; i < i32(arrayLength(&triangles)); i = i + 1){ 
        let triangle_index = u32(i);
        let triangle_hit: vec4<f32> = triangle_hit(ray, triangle_index);
        let t = triangle_hit.x;
        let N = vec3<f32>(triangle_hit.y, triangle_hit.z, triangle_hit.w);
        if all(triangle_hit != vec4<f32>(-1.0)) {
            if t > min_t && t < response.x {
                response.x = t;
                response.y = 0f;
            }
        }
    }
    if response.x == init_max_t {
        response.x = -1.;
    }
    return array(response, N);
}

fn random_vec3(seed: f32, N: vec3<f32>) -> vec3<f32> {
    var unit: vec3<f32>;
    var current_seed = seed;
    loop { 
        unit = vec3<f32>(prng(current_seed), prng(current_seed + 1.), prng(current_seed + 2.));
        if dot(unit, unit) >= 1. {
            current_seed += 3.0;
            continue;
        }
        else {
            break;
        }
    }
    
    let random_vector = normalize(unit);
    if dot(random_vector, N) > 0.0 {
        return random_vector;
    }
    else {
        return -random_vector;
    }
}


fn ray_color(ray: Ray, seed: vec4<f32>) -> vec3<f32> {
    var output_color: vec3<f32> = vec3<f32>(1.);
    var depth: i32 = 5;

    var current_ray = ray;
    while depth > 0 {
        let hit_response = has_hit(current_ray);
        let t = hit_response[0].x;

        if t > 0.0 {
            let hit_point = current_ray.orig + current_ray.dir * t;
            let N = hit_response[1];
            let material = u32(hit_response[0].y);

            var new_target: vec3<f32>;
            if material == 0u {
                new_target = hit_point + N + random_vec3(seed.x, N);
            }
            else {
                // let N_offset = normalize(50.0 * N + random_vec3(seed.x, N));
                let N_offset = N;
                new_target = hit_point + current_ray.dir - 2.0*dot(N_offset, current_ray.dir) * N_offset;
            }
            var new_ray: Ray;
            new_ray.screen_x = ray.screen_x;
            new_ray.screen_y = ray.screen_y;
            new_ray.dir = normalize(new_target - hit_point);
            new_ray.orig = hit_point;

            current_ray = new_ray;

            output_color *= 0.7;
        }
        else if t == -1.0 {
            let unit_direction = normalize(current_ray.dir);
            let coeff = 0.5*(unit_direction.y + 1.0);
            output_color *= (1.0-coeff)*vec3<f32>(1.0, 1.0, 1.0) + coeff*vec3<f32>(0.5, 0.7, 1.0);
            return output_color;
        }
        // else if t < 0.0 {
        //     return vec3<f32>(1., 0., 0.);
        // }

        depth -= 1;
    }

    return output_color;


}

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>,
        @builtin(num_workgroups) workgroups: vec3<u32>) {
    let ray_index = global_id.x + global_id.y * workgroups.x;
    let texture_size: vec2<u32> = textureDimensions(noise_texture);
    let noise_y: u32 = u32(floor(f32(global_id.x) / f32(texture_size.x)));
    let noise_x: u32 = global_id.x - noise_y * texture_size.x;

    let seed: vec4<f32> = textureLoad(noise_texture, vec2<u32>(global_id.x, global_id.y), 0)*100000f;
    var ray: Ray = v_indices[ray_index];
    let RAY_ORIGIN_DIR = ray.dir;

    let delta_u: vec3<f32> = pixel_delta_u;
    let delta_v: vec3<f32> = pixel_delta_v;

    var output_color = vec3<f32>(0.);
    let SAMPLES = 1;

    for(var sample_index: i32 = 1; sample_index < SAMPLES + 1; sample_index++) {
        ray.dir = RAY_ORIGIN_DIR + pixel_delta_u * (prng(seed.x * f32(sample_index)) - 1f) / 2f + pixel_delta_v * (prng(seed.y * f32(sample_index)) - 1f) / 2f;
        output_color += ray_color(ray, seed * f32(sample_index)) / f32(SAMPLES);
    }

    v_indices[ray_index].color = output_color;
    let s = triangles[0];
}
struct Ray {
    orig: vec3<f32>,
    dir: vec3<f32>,
    color: vec3<f32>,
    screen_x: u32,
    screen_y: u32,
}

struct Ball {
    center: vec3<f32>,
    radius: f32,
    material: u32,
}

struct Triangle {
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


// @group(0)
// @binding(2)
// var<storage> triangles: array<Triangle>;

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

// fn triangle_hit(ray: Ray, triangle_id: u32) -> f32 {
//     let kEpsilon = 0.0001;
//     let triangle: Triangle = triangles[triangle_id];
//     let v0v1 = triangle.v1 - triangle.v0;
//     let v0v2 = triangle.v2 - triangle.v0;
//     let N = cross(v0v1, v0v2);
//     let area2 = length(N);

//     if abs(dot(N, ray.dir)) < kEpsilon {
//         return -1.0;
//     }

//     let d = -dot(N, triangle.v0);
//     let t = -(dot(N, ray.orig) + d)/ dot(N, ray.dir);

//     if t < 0.0 {
//         return -1.0;
//     }

//     let P = ray.orig + ray.dir * t;

//     let edge0 = triangle.v1 - triangle.v0;
//     let vp0 = P - triangle.v0;
//     var C: vec3<f32> = cross(edge0, vp0);
//     if dot(N, C) < 0.0 {
//         return -1.0;
//     }

//     let edge1 = triangle.v2 - triangle.v1;
//     let vp1 = P - triangle.v1;
//     C = cross(edge1, vp1);
//     if dot(N, C) < 0.0 {
//         return -1.0;
//     }

//     let edge2 = triangle.v0 - triangle.v2;
//     let vp2 = P - triangle.v2;
//     C = cross(edge2, vp2);
//     if dot(N, C) < 0.0 {
//         return -1.0;
//     }

//     return t;
// }

fn has_hit(ray: Ray) -> vec2<f32> {
    let init_max_t = f32(100000000);
    let min_t: f32 = 0.00000001;
    var max_t: f32 = init_max_t;
    var ball_index: i32;

    for (var i: i32 = 0; i < i32(arrayLength(&balls)); i = i + 1){
        let ball = balls[i];
        let oc: vec3<f32> = ray.orig - ball.center;
        let a = dot(ray.dir, ray.dir);
        let half_b = dot(oc, ray.dir);
        let c = dot(oc, oc) - ball.radius * ball.radius;
        let discr: f32 = half_b * half_b - a * c;
        if discr > 0.0 {
            var solution = (-half_b - sqrt(discr)) / a;
            if solution > min_t && solution < max_t {
                max_t = solution;
                ball_index = i;
            }

            solution = (-half_b + sqrt(discr)) / a;
            if solution > min_t && solution < max_t {
                max_t = solution;
                ball_index = i;
            }
        }
    }
    if max_t == init_max_t {
        return vec2<f32>(-1.0, -1.0);
    }
    else {
        return vec2<f32>(f32(ball_index), max_t);
    }
}

fn random_vec3(seed: f32, N: vec3<f32>) -> vec3<f32> {
    var unit: vec3<f32>;
    var seed = seed;
    loop { 
        unit = vec3<f32>(prng(seed), prng(seed + 1.), prng(seed + 2.));
        if dot(unit, unit) >= 1. {
            seed += 3.0;
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
        return random_vector;
    }
}


fn ray_color(ray: Ray, seed: vec4<f32>) -> vec3<f32> {
    var output_color: vec3<f32> = vec3<f32>(1.);
    var depth: i32 = 5;

    var current_ray = ray;
    while depth > 0 {
        let has_hit_response = has_hit(current_ray);
        let t = has_hit_response.y;

        if t > 0.0 {
            let ball_index = u32(has_hit_response.x);
            let ball = balls[ball_index];
            let hit_point = current_ray.orig + current_ray.dir * (t - 0.000000001);
            let N = normalize(hit_point - ball.center);

            var new_target: vec3<f32>;
            if ball.material == 0u {
                new_target = normalize(hit_point + N + random_vec3(seed.x, N));
            }
            else {
                // let N_offset = normalize(50.0 * N + random_vec3(seed.x, N));
                let N_offset = N;
                new_target = current_ray.dir - 2.0*dot(N_offset, current_ray.dir) * N_offset;
            }
            var new_ray: Ray;
            new_ray.screen_x = ray.screen_x;
            new_ray.screen_y = ray.screen_y;
            new_ray.dir = new_target - hit_point;
            new_ray.orig = hit_point;
            current_ray = new_ray;

            output_color *= 0.5;
        }
        else if t == -1.0 {
            let unit_direction = normalize(current_ray.dir);
            let coeff = 0.5*(unit_direction.y + 1.0);
            output_color *= (1.0-coeff)*vec3<f32>(1.0, 1.0, 1.0) + coeff*vec3<f32>(0.5, 0.7, 1.0);
            return output_color;
        }

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
    let ray: Ray = v_indices[ray_index];

    v_indices[ray_index].color = ray_color(ray, seed);
}
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
}


@group(0)
@binding(0)
var<storage, read_write> v_indices: array<Ray>; // this is used as both input and output for convenience

@group(0)
@binding(1)
var<storage> balls: array<Ball>;

@group(1) @binding(0)
var noise_texture: texture_2d<f32>;


fn pcg(v: u32) -> u32 {
//   let state: u32 = v * 747796405u + 2891336453u;
//   let word: u32 = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
//   return (word >> 22u) ^ word;

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

fn has_hit(ray: Ray) -> vec2<f32> {
    let init_max_t = f32(1000000000);
    let min_t: f32 = 0.00001;
    var max_t: f32 = init_max_t;
    var ball_index: i32;

    for (var i: i32 = 0; i < i32(arrayLength(&balls)); i = i + 1){
        let ball = balls[i];
        let oc: vec3<f32> = ray.orig - ball.center;
        let a = dot(ray.dir, ray.dir);
        let b = 2.0 * dot(oc, ray.dir);
        let c = dot(oc, oc) - ball.radius * ball.radius;
        let discr: f32 = b*b - 4.0 * a * c;
        if discr > 0.0 {
            let solution = (-b - sqrt(discr)) / (2.0 * a);
            if solution > min_t && solution < max_t {
                max_t = solution;
                ball_index = i;
            }
            else {
                let solution = (-b + sqrt(discr)) / (2.0 * a);
                if solution > min_t && solution < max_t {
                    max_t = solution;
                    ball_index = i;
                }
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

fn random_vec3(seed: f32) -> vec3<f32> {
    var unit: vec3<f32>;
    loop { 
        unit = vec3<f32>(prng(seed), prng(seed + 1.), prng(seed + 2.));
        if dot(unit, unit) >= 1. {
            continue;
        }
        else {
            break;
        }
    }
    return unit;
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
            let hit_point = current_ray.orig + current_ray.dir * t;
            let N = normalize(hit_point - ball.center);

            let new_target = normalize(hit_point + N + random_vec3(seed.x));
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
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let texture_size: vec2<u32> = textureDimensions(noise_texture);
    let noise_y: u32 = u32(floor(f32(global_id.x) / f32(texture_size.x)));
    let noise_x: u32 = global_id.x - noise_y * texture_size.x;

    let seed: vec4<f32> = textureLoad(noise_texture, vec2<u32>(noise_x, noise_y), 0)*100000f;
    let ray: Ray = v_indices[global_id.x];

    // for (var i: i32 = 0; i < i32(arrayLength(&balls)); i++){
    //     let ball = balls[i];
    //     let t = has_hit(ball.center, ball.radius, ray);

    //     if t > 0.0 {
    //         let hit_point = ray.orig + ray.dir * t;
    //         let N = normalize(0.5 * (hit_point - ball.center + 1.0));
    //         v_indices[global_id.x].color = N;
    //         return;
    //         // v_indices[global_id.x].color = vec3<f32>(ball_radius, ball_radius, ball_radius);
    //     }

    // }
    v_indices[global_id.x].color = ray_color(ray, seed);
}
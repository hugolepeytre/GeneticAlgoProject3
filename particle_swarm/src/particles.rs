const GENERATIONS : usize = 50000;
const PARTICLES : usize = 75;

const B_LOW : f32 = 0.0;
const B_UP : f32 = 2.0;
const SPAN : f32 = B_UP - B_LOW;
const SPEED_PERSISTENCE_BEGIN : f32 = 0.95;
const SPEED_PERSISTENCE_END : f32 = 0.35;
const G_BEST_ATTRACT : f32 = 1.5;
const SELF_BEST_ATTRACT : f32 = 2.5;

use crate::job_list::{Ordering, Jobs};

use std::usize::MAX;
use std::f32::MIN;
use rand::prelude::*;
use itertools::izip;

pub struct Swarm {
    particles : Vec<Particle>,
    best_pos : Vec<f32>,
    best_part : usize,
    best_time : usize,
    speed_persistence : f32,
}

impl Swarm {
    pub fn random(j : &Jobs) -> Swarm {
        let mut best_part = 0;
        let mut best_pos = Vec::new();
        let mut best_time = MAX;
        let particles = (0..PARTICLES).map(|i| {
            let p = Particle::random(j);
            if p.best_end_time() < best_time {
                best_part = i;
                best_time = p.best_end_time();
                best_pos = p.best_pos();
            }; 
            p
        }).collect();
        let s = Swarm {particles, best_pos, best_part, best_time, speed_persistence : SPEED_PERSISTENCE_BEGIN};
        return s
    }

    pub fn run(&mut self, jobs : &Jobs) {
        for i in 0..GENERATIONS {
            if i > GENERATIONS/5 {
                self.speed_persistence = ((GENERATIONS-i) as f32*SPEED_PERSISTENCE_BEGIN + i as f32*SPEED_PERSISTENCE_END)/GENERATIONS as f32;
            }
            self.step(jobs);
            println!("Gen {} : {}", i + 1, self.best_time());
        }
    }

    pub fn step(&mut self, j : &Jobs) {
        for i in 0..self.particles.len() {
            let mut tmp_best_pos = self.best_pos.clone();
            let mut tmp_best_time = self.best_time;
            let mut tmp_best_part = self.best_part;
            let new_p_best = self.particles[i].step(j, &self.best_pos, self.speed_persistence);
            if new_p_best < tmp_best_time {
                tmp_best_part = i;
                tmp_best_time = new_p_best;
                tmp_best_pos = self.particles[i].best_pos();
            }
            self.best_pos = tmp_best_pos;
            self.best_part = tmp_best_part;
            self.best_time = tmp_best_time;
        }
    }

    pub fn best_time(&self) -> usize {
        return self.best_time
    }

    pub fn best_solution<'a>(&self, j : &'a Jobs) -> Ordering<'a> {
        return self.particles[self.best_part].generate_best_solution(j)
    }
}

pub struct Particle {
    speed : Vec<f32>,
    pos : Vec<f32>,
    self_best_pos : Vec<f32>,
    self_best_time : usize,
}

impl Particle {
    pub fn new(speed : Vec<f32>, pos : Vec<f32>, j : &Jobs) -> Particle {
        let self_best_pos = pos.clone();
        let mut p = Particle {speed, pos, self_best_pos, self_best_time : MAX};
        let b_t = p.eval(j);
        p.self_best_time = b_t;
        return p
    }

    pub fn random(j : &Jobs) -> Particle {
        let mut rng = thread_rng();
        let pos = (0..j.n_jobs()*j.n_machines()).map(|_| rng.gen_range(B_LOW, B_UP)).collect();
        let speed = (0..j.n_jobs()*j.n_machines()).map(|_| rng.gen_range(-SPAN, SPAN)).collect();
        return Self::new(speed, pos, j)
    }

    fn eval(&self, j : &Jobs) -> usize {
        let s = self.generate_solution(j);
        return s.end_time()
    }

    fn step(&mut self, j : &Jobs, g_pos : &Vec<f32>, s_p : f32) -> usize {
        let mut rng = thread_rng();
        self.speed = izip!(self.speed.iter(), self.pos.iter(), self.self_best_pos.iter(), g_pos.iter()).map(|(&sp, &p, &s_b_p, &b_p)| {
            let (rd1, rd2) = (rng.gen_range(0.0, 1.0), rng.gen_range(0.0, 1.0));
            minf(maxf(s_p * sp + SELF_BEST_ATTRACT * rd1 * (s_b_p - p) + G_BEST_ATTRACT * rd2 * (b_p - p), -SPAN), SPAN)
        }).collect();
        self.pos = izip!(self.pos.iter(), self.speed.iter()).map(|(&p, &s)| p + s).collect();
        let new_time = self.eval(j);
        if new_time < self.self_best_time {
            self.self_best_time = new_time;
            self.self_best_pos = self.pos.clone();
        }
        return self.self_best_time
    }

    pub fn best_end_time(&self) -> usize{
        return self.self_best_time
    }

    fn generate_solution<'a>(&self, j : &'a Jobs) -> Ordering<'a> {
        let mut task_order = Vec::new();
        let mut next_tasks = vec![0; j.n_jobs()];
        let mut n_tasks_handled = 0;
        while n_tasks_handled < self.pos.len() {
            let (job_chosen, _) = next_tasks.iter().enumerate().fold((MAX, MIN), |(chosen_job, max_found), (i, &next_task)| {
                if next_task < j.n_machines() {
                    let idx = i * j.n_machines() + next_task;
                    let pos = self.pos[idx];
                    if pos > max_found {
                        (i, pos)
                    }
                    else {
                        (chosen_job, max_found)
                    }
                }
                else {
                    (chosen_job, max_found)
                }
            });
            task_order.push((job_chosen, next_tasks[job_chosen]));
            next_tasks[job_chosen] = next_tasks[job_chosen] + 1;
            n_tasks_handled = n_tasks_handled + 1;
        }
        return Ordering::new(task_order, j)
    }

    fn generate_best_solution<'a>(&self, j : &'a Jobs) -> Ordering<'a> {
        let mut task_order = Vec::new();
        let mut next_tasks = vec![0; j.n_jobs()];
        let mut n_tasks_handled = 0;
        while n_tasks_handled < self.self_best_pos.len() {
            let (job_chosen, _) = next_tasks.iter().enumerate().fold((MAX, MIN), |(chosen_job, max_found), (i, &next_task)| {
                if next_task < j.n_machines() {
                    let idx = i * j.n_machines() + next_task;
                    let pos = self.self_best_pos[idx];
                    // println!("{}", pos);
                    if pos > max_found {
                        (i, pos)
                    }
                    else {
                        (chosen_job, max_found)
                    }
                }
                else {
                    (chosen_job, max_found)
                }
            });
            task_order.push((job_chosen, next_tasks[job_chosen]));
            next_tasks[job_chosen] = next_tasks[job_chosen] + 1;
            n_tasks_handled = n_tasks_handled + 1;
        }
        return Ordering::new(task_order, j)
    }

    fn best_pos(&self) -> Vec<f32> {
        return self.self_best_pos.clone()
    }
}

fn minf(a : f32, b : f32) -> f32 {
    if a < b {a} else {b}
}

fn maxf(a : f32, b : f32) -> f32 {
    if a > b {a} else {b}
}
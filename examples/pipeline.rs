// https://medium.com/@polyglot_factotum/rust-concurrency-patterns-natural-born-pipelines-4d599e7612fc
use std::collections::VecDeque;
use std::sync::mpsc::{channel, Sender};
use std::thread;

enum PipelineMsg {
    Generated(usize),
    Squared(usize),
    Merged(usize),
}

fn generate(num_chan: Sender<PipelineMsg>) {
    let mut num = 2;
    let _ = thread::Builder::new().spawn(move || {
        while let Ok(_) = num_chan.send(PipelineMsg::Generated(num)) {
            num = num + 1;
        }
    });
}

fn square(sqr_chan: Sender<PipelineMsg>) -> Sender<PipelineMsg> {
    let (chan, port) = channel();

    let _ = thread::Builder::new().spawn(move || {
        for msg in port {
            let num = match msg {
                PipelineMsg::Generated(num) => num,
                _ => panic!("unexpected msg rcvd at sqr stg"),
            };

            let _ = sqr_chan.send(PipelineMsg::Squared(num * num));
        }
    });

    chan
}

fn merge(merged_chan: Sender<PipelineMsg>) -> Sender<PipelineMsg> {
    let (chan, port) = channel();
    let _ = thread::Builder::new().spawn(move || {
        for msg in port {
            let squared = match msg {
                PipelineMsg::Squared(num) => num,
                _ => panic!("unexpected ms rcvd at mrg stg"),
            };

            let _ = merged_chan.send(PipelineMsg::Merged(squared));
        }
    });

    chan
}

fn main() {
    // channel for results
    //  (tx, rx)
    let (results_chan, results_port) = channel();

    // channel for generated numbers
    let (gen_chan, gen_port) = channel();

    // "forward" anything sent to merged_chan to the results chan
    let merged_chan = merge(results_chan);

    {
        // Create workers which square the number and send to the merged chan
        // NOTE we move the merged sender to the square worker so that it will
        // go out of scope when the for loop in square exits
        let mut sqr_workers: VecDeque<Sender<PipelineMsg>> = vec![
            square(merged_chan.clone()),
            square(merged_chan.clone()),
            square(merged_chan),
        ].into_iter()
            .collect();

        // start generating numbers
        generate(gen_chan);

        // Read generated numbers (Iterator trait)
        for msg in gen_port {
            let gen_num = match msg {
                PipelineMsg::Generated(num) => num,
                _ => panic!("dunno 1"),
            };

            // Get a worker from the front
            let worker = sqr_workers.pop_front().unwrap();
            // send the generated number to the square worker
            let _ = worker.send(msg);
            // Push the worker to the back of the queue. Cycling the workers to keep them busy
            sqr_workers.push_back(worker);
            // Stop reading from gen_port after we've generated a few numbers
            if gen_num == 30_000 {
                break;
            }
        }

        // Square workers (Senders) go out of scope here
    }

    // Read the results
    for result in results_port {
        match result {
            PipelineMsg::Merged(num) => println!("{}", num),
            // PipelineMsg::Merged(_) => continue,
            _ => panic!("unexpctj resut"),
        }
    }
}

use tev_client::{PacketCloseImage, PacketCreateImage, PacketUpdateImage, TevClient, TevPacket};

use crate::common::progress::{Block, PixelResult, ProgressHandler};

pub struct TevProgress {
    name: String,
    client: Option<TevClient>,
}

impl TevProgress {
    pub fn new(name: &str, client: TevClient) -> Self {
        TevProgress { name: name.into(), client: Some(client) }
    }

    pub fn try_send<'s, P: TevPacket + 's>(&'s mut self, packet: impl FnOnce(&'s str) -> P) {
        if let Some(client) = &mut self.client {
            let packet = packet(&self.name);
            if let Err(e) = client.send(packet) {
                println!("Communication with tev failed, future commands will not be sent.\n{:?}", e);
                self.client = None;
            }
        }
    }
}

impl ProgressHandler for TevProgress {
    type State = Self;

    fn init(mut self, width: u32, height: u32) -> Self::State {
        self.try_send(|image_name| PacketCloseImage {
            image_name
        });
        self.try_send(|image_name| PacketCreateImage {
            image_name,
            grab_focus: false,
            width,
            height,
            // TODO send variance, samples, ... as well
            channel_names: &["R", "G", "B"],
        });

        self
    }

    fn update(state: &mut Self::State, block: Block, pixels: &Vec<PixelResult>) {
        //transform data into format expected by tev
        let mut data = Vec::with_capacity(3 * pixels.len());
        for dy in 0..block.height {
            for dx in 0..block.width {
                let p = pixels[(dx + dy * block.width) as usize];
                data.extend_from_slice(&[p.color.red, p.color.green, p.color.blue])
            }
        }

        state.try_send(|image_name| {
            PacketUpdateImage {
                image_name,
                grab_focus: false,
                channel_names: &["R", "G", "B"],
                channel_offsets: &[0, 1, 2],
                channel_strides: &[3, 3, 3],
                x: block.x,
                y: block.y,
                width: block.width,
                height: block.height,
                data: &data,
            }
        })
    }
}
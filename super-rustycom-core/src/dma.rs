pub struct Dma {
    mdma_en: u8,
    hdma_en: u8,
    dma_p: [u8; 8],
    bb_ad: [u8; 8],
    a1t_l: [u8; 8],
    a1t_h: [u8; 8],
    a1_b: [u8; 8],
    das_l: [u8; 8],
    das_h: [u8; 8],
    das_b: [u8; 8],
    a2a_l: [u8; 8],
    a2a_h: [u8; 8],
    ntr_l: [u8; 8],
    unused: [u8; 8],
}

impl Dma {
    pub fn new() -> Dma {
        Dma {
            mdma_en: 0x00,
            hdma_en: 0x00,
            dma_p: [0; 8],
            bb_ad: [0; 8],
            a1t_l: [0; 8],
            a1t_h: [0; 8],
            a1_b: [0; 8],
            das_l: [0; 8],
            das_h: [0; 8],
            das_b: [0; 8],
            a2a_l: [0; 8],
            a2a_h: [0; 8],
            ntr_l: [0; 8],
            unused: [0; 8],
        }
    }

    #[allow(dead_code)]
    pub fn read_mdma_en(&self) -> u8 {
        self.mdma_en
    }

    pub fn write_mdma_en(&mut self, value: u8) {
        self.mdma_en = value; // TODO: Do transfer
    }

    #[allow(dead_code)]
    pub fn read_hdma_en(&self) -> u8 {
        self.hdma_en
    }

    pub fn write_hdma_en(&mut self, value: u8) {
        self.hdma_en = value;
    }

    pub fn read(&self, addr: usize) -> u8 {
        let port = addr & 0x000F;
        let channel = (addr & 0x00F0) >> 4;
        match port {
            DMAPX => self.dma_p[channel],
            BBADX => self.bb_ad[channel],
            A1TXL => self.a1t_l[channel],
            A1TXH => self.a1t_h[channel],
            A1BX => self.a1_b[channel],
            DASXL => self.das_l[channel],
            DASXH => self.das_h[channel],
            DASBX => self.das_b[channel],
            A2AXL => self.a2a_l[channel],
            A2AXH => self.a2a_h[channel],
            NTRLX => self.ntr_l[channel],
            UNUSEDX => self.unused[channel],
            MIRRX => self.unused[channel],
            _ => panic!("DMA read ${:04X}: Unused region", addr),
        }
    }

    pub fn write(&mut self, addr: usize, value: u8) {
        let port = addr & 0x000F;
        let channel = (addr & 0x00F0) >> 4;
        match port {
            DMAPX => self.dma_p[channel] = value,
            BBADX => self.bb_ad[channel] = value,
            A1TXL => self.a1t_l[channel] = value,
            A1TXH => self.a1t_h[channel] = value,
            A1BX => self.a1_b[channel] = value,
            DASXL => self.das_l[channel] = value,
            DASXH => self.das_h[channel] = value,
            DASBX => self.das_b[channel] = value,
            A2AXL => self.a2a_l[channel] = value,
            A2AXH => self.a2a_h[channel] = value,
            NTRLX => self.ntr_l[channel] = value,
            UNUSEDX => self.unused[channel] = value,
            MIRRX => self.unused[channel] = value,
            _ => panic!("DMA write ${:04X}: Unused region", addr),
        }
    }
}

// Port types
const DMAPX: usize = 0x0000;
const BBADX: usize = 0x0001;
const A1TXL: usize = 0x0002;
const A1TXH: usize = 0x0003;
const A1BX: usize = 0x0004;
const DASXL: usize = 0x0005;
const DASXH: usize = 0x0006;
const DASBX: usize = 0x0007;
const A2AXL: usize = 0x0008;
const A2AXH: usize = 0x0009;
const NTRLX: usize = 0x000A;
const UNUSEDX: usize = 0x000B;
const MIRRX: usize = 0x000F;

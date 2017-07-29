pub struct Dma {
    mdma_en: u8,
    hdma_en: u8,
    dma_p:  [u8; 8],
    bb_ad:  [u8; 8],
    a1t_l:  [u8; 8],
    a1t_h:  [u8; 8],
    a1_b:   [u8; 8],
    das_l:  [u8; 8],
    das_h:  [u8; 8],
    das_b:  [u8; 8],
    a2a_l:  [u8; 8],
    a2a_h:  [u8; 8],
    ntr_l:  [u8; 8],
    unused: [u8; 8],
}

impl Dma {
    pub fn new() -> Dma {
        Dma {
            mdma_en: 0x00,
            hdma_en: 0x00,
            dma_p:  [0; 8],
            bb_ad:  [0; 8],
            a1t_l:  [0; 8],
            a1t_h:  [0; 8],
            a1_b:   [0; 8],
            das_l:  [0; 8],
            das_h:  [0; 8],
            das_b:  [0; 8],
            a2a_l:  [0; 8],
            a2a_h:  [0; 8],
            ntr_l:  [0; 8],
            unused: [0; 8],
        }
    }

    pub fn read_mdma_en(&self) -> u8 {
        self.mdma_en
    }

    pub fn write_mdma_en(&mut self, value: u8) {
        self.mdma_en = value;// TODO: Do transfer
    }

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
            DMAPx   => self.dma_p[channel],
            BBADx   => self.bb_ad[channel],
            A1TxL   => self.a1t_l[channel],
            A1TxH   => self.a1t_h[channel],
            A1Bx    => self.a1_b[channel],
            DASxL   => self.das_l[channel],
            DASxH   => self.das_h[channel],
            DASBx   => self.das_b[channel],
            A2AxL   => self.a2a_l[channel],
            A2AxH   => self.a2a_h[channel],
            NTRLx   => self.ntr_l[channel],
            UNUSEDx => self.unused[channel],
            MIRRx   => self.unused[channel],
            _       => panic!("DMA read ${:04X}: Unused region", addr)
        }
    }

    pub fn write(&mut self, value: u8, addr: usize) {
        let port = addr & 0x000F;
        let channel = (addr & 0x00F0) >> 4;
        match port {
            DMAPx   => self.dma_p[channel] = value,
            BBADx   => self.bb_ad[channel] = value,
            A1TxL   => self.a1t_l[channel] = value,
            A1TxH   => self.a1t_h[channel] = value,
            A1Bx    => self.a1_b[channel] = value,
            DASxL   => self.das_l[channel] = value,
            DASxH   => self.das_h[channel] = value,
            DASBx   => self.das_b[channel] = value,
            A2AxL   => self.a2a_l[channel] = value,
            A2AxH   => self.a2a_h[channel] = value,
            NTRLx   => self.ntr_l[channel] = value,
            UNUSEDx => self.unused[channel] = value,
            MIRRx   => self.unused[channel] = value,
            _       => panic!("DMA write ${:04X}: Unused region", addr)
        }
    }
}

// Port types
const DMAPx: usize    = 0x0000;
const BBADx: usize    = 0x0001;
const A1TxL: usize    = 0x0002;
const A1TxH: usize    = 0x0003;
const A1Bx: usize     = 0x0004;
const DASxL: usize    = 0x0005;
const DASxH: usize    = 0x0006;
const DASBx: usize    = 0x0007;
const A2AxL: usize    = 0x0008;
const A2AxH: usize    = 0x0009;
const NTRLx: usize    = 0x000A;
const UNUSEDx: usize  = 0x000B;
const MIRRx: usize    = 0x000F;

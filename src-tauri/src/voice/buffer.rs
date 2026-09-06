// Du son en memoire : des echantillons mono, et la frequence a laquelle les lire.
//
// MONO PARTOUT. Le decaleur de hauteur et de formants ne travaille que sur un canal, et le
// moteur ramene de toute facon toute reference en mono avant de l'encoder : porter deux canaux
// jusqu'ici aurait double la memoire pour les jeter a la fin.

pub struct Audio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

// `Debug` A LA MAIN, et pas derive. Une reference de trente secondes porte 720 000 flottants :
// un `{:?}` derive les deverserait tous dans le message d'un test qui echoue, ce qui rend
// illisible exactement le moment ou l'on a besoin de lire.
impl std::fmt::Debug for Audio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Audio({} echantillons a {} Hz)", self.samples.len(), self.sample_rate)
    }
}

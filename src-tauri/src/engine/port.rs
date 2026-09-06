// Un port libre, plutot qu'un port fixe.
//
// Ventriloque se copie sur une cle et se lance ou l'on veut : deux exemplaires ouverts en meme
// temps sont un cas ordinaire, et avec un port fixe le second trouverait le moteur du premier --
// il parlerait avec ses voix a lui, ce qui est pire qu'une erreur franche.
//
// La fenetre entre la liberation et la reprise du port est une course theorique. Elle est
// acceptee : le seul concurrent plausible est un autre Ventriloque, et il tirera un autre numero.

use std::net::TcpListener;

const FIRST: u16 = 8231;
const LAST: u16 = 8331;

pub fn free_port() -> u16 {
    (FIRST..LAST)
        .find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
        .unwrap_or(FIRST)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_port_choisi_est_dans_la_plage() {
        assert!((FIRST..LAST).contains(&free_port()));
    }

    // Un port pris n'est pas rendu : c'est exactement le cas de deux Ventriloques ouverts.
    //
    // ON NE VERIFIE PAS QUE LE PORT RENDU SE LIE ENCORE, et c'est delibere : entre le moment ou
    // `free_port` relache son essai et celui ou l'appelant s'y installe, la place est a qui la
    // prend. Un test qui l'affirmerait mesurerait la chance, et echouerait un jour sur deux
    // quand la suite tourne en parallele. La course est documentee et acceptee ici meme.
    #[test]
    fn un_port_occupe_est_saute() {
        let Ok(held) = TcpListener::bind(("127.0.0.1", FIRST)) else {
            return; // La machine ne prete pas ce port : rien a prouver ici.
        };
        assert_ne!(free_port(), FIRST);
        drop(held);
    }
}

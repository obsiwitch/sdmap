module kbd_frame(size, border, n, m) {
    module buttons(size, margin, n, m) {
        btnSize = [(size[0] - margin[0]*(n-1))/n, (size[1] - margin[1]*(m-1))/m, size[2]];
        for(i=[0: n-1]) {
            for(j=[0: m-1]) {
                translate([i*(btnSize[0] + margin[0]), j*(btnSize[1] + margin[1]), 0]) cube(btnSize);
            }
        }
    }

    difference() {
        cube([size[0] + 2*border[0], size[1] + 2*border[1], size[2]]);
        translate(border) buttons(size=size, margin=border, n=n, m=m);
    }
    translate([0, 0, -1]) difference() {
        cube([size[0] + 2*border[0], size[1] + 2*border[1], 1]);
        translate(border) cube(size);
    }
}

kbd_frame(size=[32, 32, 1], border=[1, 1, 0], n=3, m=4);

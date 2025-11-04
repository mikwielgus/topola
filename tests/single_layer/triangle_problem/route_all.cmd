{
  "done": [
    {
      "MultilayerAutoroute": [
        [
          {
            "pin": "P1_1-1",
            "layer": "F.Cu"
          },
          {
            "pin": "P1_2-1",
            "layer": "F.Cu"
          },
          {
            "pin": "P2_1-1",
            "layer": "F.Cu"
          },
          {
            "pin": "P2_2-1",
            "layer": "F.Cu"
          },
          {
            "pin": "P3_1-1",
            "layer": "F.Cu"
          },
          {
            "pin": "P3_2-1",
            "layer": "F.Cu"
          }
        ],
        {
          "anterouter": {
            "fanout_clearance": 200.0
          },
          "planar": {
            "principal_layer": 0,
            "presort_by": "RatlineIntersectionCountAndLength",
            "permutate": true,
            "router": {
              "routed_band_width": 100.0,
              "wrap_around_bands": true,
              "squeeze_through_under_bends": true
            },
            "timeout": {
              "initial": 1.0,
              "progress_bonus": 0.005
            }
          },
          "timeout": {
            "initial": 5.0,
            "progress_bonus": 0.5
          }
        }
      ]
    }
  ],
  "undone": []
}
# Auto-aim gimbal ownership contract

When `SubscribeAutoAim` is enabled, the active external command transport
(`udp`, `talos`, or `ros2`) is the sole writer of the controlled vehicle's
gimbal transform and `InfantryGimbal` state.

Manual keyboard and mouse gimbal systems must return without reading or
rewriting that transform while auto aim is active. Bevy system order is not an
ownership mechanism: two unordered writers can expose different Euler branches
to capture/runtime feedback and make pitch jump between the intended command
and a pitch-limit value.

Transport pitch contracts:

- every command transport uses the same absolute gimbal elevation in degrees;
- `pitch_deg == 0` is level aim, positive raises the optical axis, and negative lowers it;
- UDP, ROS2 and Talos shared-memory commands apply the configured symmetric
  pitch limit without any transport-specific offset or sign conversion;
- exactly one Daedalus transport applies commands, selected by
  `config.auto_aim.command_transport`.

Do not compensate for an ownership conflict with pitch offsets, wider limits,
tracker changes, or ballistic tuning. Validate a static target by confirming
that commanded and runtime local pitch remain on one continuous branch.

const std = @import("std");
const net = std.net;
const print = std.debug.print;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();

    const loopback = try net.Ip4Address.parse("0.0.0.0", 34254);
    const localhost = net.Address{ .in = loopback };
    var server = try localhost.listen(.{
        .reuse_address = true,
    });
    defer server.deinit();

    const addr = server.listen_address;
    print("Listening on {}, access this port to end the program\n", .{addr.getPort()});

    var client = try server.accept();
    defer client.stream.close();

    print("Connection received! {} is sending data.\n", .{client.address});
    const messageLength: [8]u8 = undefined;

    // Program flow:
    // First 8 bytes contains length of the message.
    // The rest of the bytes contains the message, with the afforementioned length
    const length = try client.stream.reader().read(messageLength);

    print("{} says message is: {s} bytes long\n", .{ client.address, length });
}

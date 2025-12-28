package server

import (
	"context"

	pb "upstream/userspb"

	"google.golang.org/grpc"
)

// UserService implements the generated gRPC interface.
type UserService struct {
	pb.UnimplementedUserServiceServer
}

func (s *UserService) GetUser(ctx context.Context, req *pb.UserRequest) (*pb.UserReply, error) {
	return &pb.UserReply{Id: req.Id}, nil
}

func RegisterUserService(s *grpc.Server) {
	pb.RegisterUserServiceServer(s, &UserService{})
}

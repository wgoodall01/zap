import { Avatar } from "@radix-ui/themes";
import type { User } from "../api_client/types.gen";

interface ProfileProps {
  size?: "1" | "2" | "3";
}

interface ProfileWithUserProps extends ProfileProps {
  user: User;
  id?: never;
}

interface ProfileWithIdProps extends ProfileProps {
  id: string;
  user?: never;
}

type ProfileComponentProps = ProfileWithUserProps | ProfileWithIdProps;

export function Profile({ size = "2", ...props }: ProfileComponentProps) {
  // If user object is provided, use it directly
  if ("user" in props && props.user) {
    const { user } = props;
    return (
      <Avatar
        size={size}
        src={user.photoUrl || undefined}
        fallback={user.name.charAt(0).toUpperCase()}
        radius="full"
      />
    );
  }

  // If only ID is provided, we would need to fetch the user data
  // For now, just return a placeholder
  if ("id" in props && props.id) {
    return <Avatar size={size} fallback="?" radius="full" />;
  }

  return null;
}

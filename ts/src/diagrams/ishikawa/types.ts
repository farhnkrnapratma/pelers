export interface IshikawaNode {
  text: string;
  children: IshikawaNode[];
}

// The root is the effect; its children are the categories of causes, theirs the causes, and so on.
export interface IshikawaModel {
  type: 'ishikawa';
  title: string | undefined;
  root: IshikawaNode | undefined;
}

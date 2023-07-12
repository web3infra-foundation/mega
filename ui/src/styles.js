//文件树展示的样式文件

export default {
    component: {
      display: "inline-block",
      margin: "20px 0 10px 0",
      verticalAlign: "top",
      
      width: "100%",
      "@media (max-width: 640px)": {
        width: "100%",
        display: "block",
        color: "pink",
        backgroundColor: "pink"
      }
    },
    ul: {
      color: "pink"
    },
    viewer: {
      base: {
        fontFamily: "lucida grande ,tahoma,verdana,arial,sans-serif"
      }
    },
    t: {
      tree: {
        base: {
          listStyle: "none",
          color: "blue",
          backgroundColor: "transparent",
          margin: 0,
          padding: 0,
          color: "pink",
          fontFamily: "lucida grande ,tahoma,verdana,arial,sans-serif",
          fontSize: "14px"
        },
        node: {
          base: {
            position: "relative",
            color: "blue",
          },
          link: {
            cursor: "pointer",
            position: "relative",
            padding: "0px 5px",
            display: "block"
          },
          activeLink: {
            background: "white"
          },
          toggle: {
            base: {
              position: "relative",
              display: "inline-block",
              verticalAlign: "top",
              marginLeft: "-5px",
              height: "24px",
              width: "24px",
              
            },
            wrapper: {
              position: "absolute",
              top: "50%",
              left: "50%",
              margin: "-7px 0 0 -7px",
              height: "14px"
            },
            height: 14,
            width: 14,
            arrow: {
              fill: "#9DA5AB",
              strokeWidth: 0
            }
          },
          header: {
            base: {
              display: "inline-block",
              verticalAlign: "top",
              color: "#9DA5AB"
            },
            connector: {
              width: "2px",
              height: "12px",
              borderLeft: "solid 2px black",
              borderBottom: "solid 2px black",
              position: "absolute",
              top: "0px",
              left: "-21px"
            },
            title: {
              lineHeight: "24px",
              verticalAlign: "middle",
              color: "#0e3e86",
            }
          },
          subtree: {
            listStyle: "none",
            paddingLeft: "19px"
          },
          loading: {
            color: "#E2C089"
          }
        }
      }
    }
  };
  